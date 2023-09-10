use crate::{
    interrupts::idt::InterruptDescriptorTable, locks::mutex::Mutex, print, vga_buffer::WRITER,
};
use lazy_static::lazy_static;
use pic8259::ChainedPics;

use super::idt::InterruptStackFrame;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.add_exceptions();
        idt.add(PIC_1_OFFSET as usize, timer_interrupt_handler as u64);
        idt.add(33, keyboard_interrupt_handler as u64);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

extern "x86-interrupt" fn timer_interrupt_handler(_: &mut InterruptStackFrame) {
    unsafe {
        PICS.lock().notify_end_of_interrupt(PIC_1_OFFSET);
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_: InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use x86_64::instructions::port::Port;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(
            Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore)
        );
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => {
                    // Backspace
                    if character == '\u{8}' {
                        let mut writer = WRITER.lock();
                        writer.write_byte(b'\x08');
                    } else {
                        print!("{}", character);
                    }
                }

                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    unsafe {
        PICS.lock().notify_end_of_interrupt(33);
    }
}