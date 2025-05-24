//! This crate provides the `libc` types for the `libc` crate.
//!
//!
#![no_std]
#![deny(warnings)]
#![deny(missing_docs)]
#![deny(clippy::all)]

#[macro_use]
extern crate bitflags;

#[macro_use]
mod utils;

mod arch;
pub mod consts;
pub mod elf;
pub mod epoll;
pub mod fcntl;
pub mod futex;
pub mod internal;
pub mod ioctl;
pub mod mman;
pub mod others;
pub mod poll;
pub mod resource;
pub mod sched;
pub mod signal;
pub mod termios;
pub mod time;
pub mod times;
pub mod types;
pub mod utsname;
