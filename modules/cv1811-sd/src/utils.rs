use crate::consts::{PresentState, SD_DRIVER_ADDR};
use bit_struct::*;

pub fn reg_transfer<T>(offset: usize) -> &'static mut T {
    unsafe { ((SD_DRIVER_ADDR + offset) as *mut T).as_mut().unwrap() }
}

/// check the sdcard that was inserted
pub fn check_sd() -> bool {
    let present_state = reg_transfer::<PresentState>(0x24);
    present_state.card_inserted().get() == u1!(1)
}

pub fn mmio_clrsetbits_32(addr: *mut u32, clear: u32, set: u32) {
    unsafe {
        *addr = (*addr & !clear) | set;
    }
}

pub fn mmio_clearbits_32(addr: *mut u32, clear: u32) {
    unsafe {
        *addr = *addr & !clear;
    }
}

pub fn mmio_setbits_32(addr: *mut u32, set: u32) {
    unsafe {
        *addr = *addr | set;
    }
}

pub fn mmio_write_32(addr: *mut u32, value: u32) {
    unsafe {
        *addr = value;
    }
}

pub fn mmio_read_32(addr: *mut u32) -> u32 {
    unsafe { *addr }
}

pub mod test {
    use logging::print;
    #[no_mangle]
    pub fn hexdump(data: &[u8]) {
        const PRELAND_WIDTH: usize = 70;
        logging::println!("{:-^1$}", " hexdump ", PRELAND_WIDTH);
        for offset in (0..data.len()).step_by(16) {
            for i in 0..16 {
                if offset + i < data.len() {
                    logging::print!("{:02x} ", data[offset + i]);
                } else {
                    logging::print!("{:02} ", "");
                }
            }

            logging::print!("{:>6}", ' ');

            for i in 0..16 {
                if offset + i < data.len() {
                    let c = data[offset + i];
                    if c >= 0x20 && c <= 0x7e {
                        logging::print!("{}", c as char);
                    } else {
                        logging::print!(".");
                    }
                } else {
                    logging::print!("{:02} ", "");
                }
            }

            logging::println!("");
        }
        logging::println!("{:-^1$}", " hexdump end ", PRELAND_WIDTH);
    }
}
