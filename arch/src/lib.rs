#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(asm_const)]

#[macro_use]
extern crate log;

mod riscv;

#[cfg(target_arch = "riscv64")]
pub use riscv::*;