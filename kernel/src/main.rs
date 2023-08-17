#![no_main]
#![no_std]
#![feature(exclusive_range_pattern)]
#![feature(drain_filter)]
#![feature(ip_in_core)]
#![feature(async_closure)]
#![feature(let_chains)]
#![feature(panic_info_message)]
#![feature(stdsimd)]

#[macro_use]
extern crate logging;
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate bitflags;

mod epoll;
mod modules;
mod panic;
mod socket;
mod syscall;
mod task_cache;
mod tasks;

use arch::enable_irq;
use devices;
use frame_allocator;
use hal;
use kalloc;

use crate::{syscall::cache_task_template, tasks::kernel::kernel_interrupt};

#[no_mangle]
fn main(hart_id: usize, device_tree: usize) {
    // if hart_id != 0 {
    //     loop {}
    // }

    extern "C" {
        fn start();
        fn end();
    }

    let str = include_str!("banner.txt");
    println!("{}", str);

    // initialize logging module
    logging::init();

    info!(
        "program size: {}KB",
        (end as usize - start as usize) / 0x400
    );

    // initialize interrupt
    hal::interrupt::init();
    hal::interrupt::reg_kernel_int(kernel_interrupt);

    // print boot info
    info!("booting at core {}", hart_id);

    // initialize kernel alloc module
    kalloc::init();

    // initialize device settings
    devices::init_device(device_tree);

    // initialize frame allocator
    frame_allocator::init();

    // get devices and init
    devices::prepare_devices();

    // initialize filesystem
    fs::init();

    // enable interrupts
    enable_irq();

    // cache task with task templates
    // cache_task_template("/bin/busybox").expect("can't cache task");
    cache_task_template("./busybox").expect("can't cache task");
    cache_task_template("busybox").expect("can't cache task");
    cache_task_template("./runtest.exe").expect("can't cache task");
    cache_task_template("entry-static.exe").expect("can't cache task");
    cache_task_template("libc.so").expect("can't cache task");

    // init kernel threads and async executor
    tasks::init();

    println!("Task All Finished!");
}
