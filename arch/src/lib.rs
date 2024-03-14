#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(asm_const)]
#![feature(stdsimd)]
#![feature(const_mut_refs)]
#![feature(const_slice_from_raw_parts_mut)]
#![cfg_attr(target_arch = "riscv64", feature(riscv_ext_intrinsics))]
#![cfg_attr(target_arch = "aarch64", feature(const_option))]

extern crate alloc;

#[macro_use]
extern crate log;

mod addr;
mod api;
// mod pte;
// pub use pte::MappingFlags;
#[cfg(target_arch = "riscv64")]
mod riscv64;

use core::mem::size_of;

use alloc::vec::Vec;
#[cfg(target_arch = "riscv64")]
pub use riscv64::*;

#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(target_arch = "aarch64")]
pub use aarch64::*;

#[cfg(target_arch = "loongarch64")]
mod loongarch64;

#[cfg(target_arch = "loongarch64")]
pub use loongarch64::*;

pub use addr::*;
pub use api::*;

// pub trait ContextOps {
//     fn set_sp(&mut self, sp: usize);
//     fn sp(&self) -> usize;
//     fn set_ra(&mut self, ra: usize);
//     fn ra(&self) -> usize;
//     fn set_sepc(&mut self, sepc: usize);
//     fn sepc(&self) -> usize;

//     fn args(&self) -> [usize; 6];
//     fn set_arg0(&mut self, ret: usize);
//     fn set_arg1(&mut self, ret: usize);
//     fn set_arg2(&mut self, ret: usize);

//     fn syscall_number(&self) -> usize;
//     fn syscall_ok(&mut self);

//     fn set_ret(&mut self, ret: usize);
//     fn set_tls(&mut self, tls: usize);
// }

#[derive(Debug)]
pub enum ContextArgs {
    SEPC,
    RA,
    SP,
    RET,
    ARG0,
    ARG1,
    ARG2,
    TLS,
    SYSCALL,
}

#[derive(Debug, Clone, Copy)]
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

const STACK_SIZE: usize = 0x80000;
const CONTEXT_SIZE: usize = size_of::<Context>();

#[link_section = ".bss.stack"]
static mut BOOT_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

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
    unsafe { INT_RECORDS.clone() }
}

pub fn clear_bss() {
    extern "C" {
        fn _sbss();
        fn _ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(_sbss as usize as *mut u128, (_ebss as usize - _sbss as usize) / size_of::<u128>())
            .fill(0);
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MappingFlags: u64 {
        const None = 0;
        const U = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const A = 1 << 4;
        const D = 1 << 5;
        const Device = 1 << 6;
        const Cache = 1 << 7;

        const URW = Self::U.bits() | Self::R.bits() | Self::W.bits();
        const URX = Self::U.bits() | Self::R.bits() | Self::X.bits();
        const URWX = Self::URW.bits() | Self::X.bits();
    }
}
