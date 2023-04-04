#![no_std]

#[macro_use]
extern crate log;

use core::arch::asm;
use core::mem::size_of;

extern "C" {
    fn stext();
    fn etext();
}

// Print the backtrace starting from the caller
pub fn backtrace() {
    unsafe {
        let mut pc;
        asm!("mv {ptr}, ra", ptr = out(reg) pc);
        let mut fp;
        asm!("mv {ptr}, fp", ptr = out(reg) fp);
        let mut stack_num = 0;

        warn!("{:=^32}", " UNWIND_START ");
        while pc >= stext as usize && pc <= etext as usize && fp as usize != 0 {
            warn!("{:#010X}", pc - size_of::<usize>(),);

            stack_num = stack_num + 1;
            fp = *(fp as *const usize).offset(-2);
            pc = *(fp as *const usize).offset(-1);
        }
        warn!("{:=^32}", " UNWIND_END ");
    }
}
