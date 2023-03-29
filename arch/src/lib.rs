#![no_std]
#![no_main]

#[macro_use]
extern crate log;

mod riscv;

#[cfg(target_arch = "riscv64")]
pub use riscv::*;