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

#[cfg(target_arch = "riscv64")]
mod riscv64;

use alloc::vec::Vec;
#[cfg(target_arch = "riscv64")]
pub use riscv64::*;

#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

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
    fn set_tp(&mut self, tp: usize);
    fn tp(&self) -> usize;

    fn set_arg0(&mut self, ret: usize);
    fn set_arg1(&mut self, ret: usize);
    fn set_arg2(&mut self, ret: usize);

    fn syscall_number(&self) -> usize;
    fn args(&self) -> [usize; 7];
    fn syscall_ok(&mut self);

    fn set_ret(&mut self, ret: usize);

    fn clear(&mut self);

    fn set_tls(&mut self, tls: usize);
}

extern "Rust" {
    fn interrupt_table() -> Option<fn(&mut Context, TrapType)>;
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