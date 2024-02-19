#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(asm_const)]
#![feature(stdsimd)]
#![feature(const_mut_refs)]
#![feature(const_slice_from_raw_parts_mut)]

extern crate alloc;

#[macro_use]
extern crate log;

mod api;
mod addr;
// mod pte;
// pub use pte::MappingFlags;
#[cfg(target_arch = "riscv64")]
mod riscv64;

use alloc::vec::Vec;
#[cfg(target_arch = "riscv64")]
pub use riscv64::*;


#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

pub use api::*;
pub use addr::*;

pub struct IntTable {
    pub timer: fn(),
}

pub trait ContextOps {
    fn set_sp(&mut self, sp: usize);
    fn sp(&self) -> usize;
    fn set_ra(&mut self, ra: usize);
    fn ra(&self) -> usize;
    fn set_sepc(&mut self, sepc: usize);
    fn sepc(&self) -> usize;

    fn args(&self) -> [usize; 6];
    fn set_arg0(&mut self, ret: usize);
    fn set_arg1(&mut self, ret: usize);
    fn set_arg2(&mut self, ret: usize);

    fn syscall_number(&self) -> usize;
    fn syscall_ok(&mut self);

    fn set_ret(&mut self, ret: usize);
    fn clear(&mut self);
    fn set_tls(&mut self, tls: usize);
}

#[derive(Debug)]
pub enum TrapType {
    Breakpoint,
    UserEnvCall,
    Time,
    Unknown,
    SupervisorExternal,
    StorePageFault(usize),
    LoadPageFault(usize),
    InstructionPageFault(usize),
    IllegalInstruction(usize),
}

pub enum MapPageSize {
    Page4k,
    Page2m,
    Page1G,
}

static mut INT_RECORDS: Vec<usize> = Vec::new();

pub fn add_irq(irq: usize) {
    unsafe {
        while INT_RECORDS.len() < 256 {
            INT_RECORDS.push(0);
        }
        INT_RECORDS[irq] += 1;
    }
}

pub fn get_int_records() -> Vec<usize> {
    // INT_RECORDS.lock().clone()
    unsafe { INT_RECORDS.clone() }
}

pub fn prepare_init() {
    ArchInterface::init_logging();
    // Init allocator
    allocator::init();
}

pub fn clear_bss() {
    extern "C" {
        fn _sbss();
        fn _ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(_sbss as usize as *mut u8, _ebss as usize - _sbss as usize)
            .fill(0);
    }
}
