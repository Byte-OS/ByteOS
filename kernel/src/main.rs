#![no_main]
#![no_std]
#![feature(exclusive_range_pattern)]
#![feature(extract_if)]
#![feature(ip_in_core)]
#![feature(async_closure)]
#![feature(let_chains)]
#![feature(panic_info_message)]
#![feature(stdsimd)]

// include modules drivers
include!(concat!(env!("OUT_DIR"), "/drivers.rs"));

#[macro_use]
extern crate logging;
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate bitflags;

mod epoll;
// mod modules;
mod panic;
mod socket;
mod syscall;
mod tasks;

use arch::enable_irq;
use devices;
use executor::FileItem;
use frame_allocator;
use fs::get_filesystem;
use hal;
use kalloc;
use vfscore::{INodeInterface, OpenFlags};

use crate::tasks::kernel::kernel_interrupt;

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

    println!("run kernel @ hart {}", hart_id);

    // initialize logging module
    logging::init(option_env!("LOG"));

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

    {
        // let cache_file = vec!["busybox", "entry-static.exe", "runtest.exe"];
        let rootfs = get_filesystem(0).root_dir();
        let tmpfs =
            FileItem::fs_open("/tmp_home", OpenFlags::O_DIRECTORY).expect("can't open /tmp_home");
        for file in rootfs.read_dir().expect("can't read files") {
            tmpfs
                .link(
                    &file.filename,
                    rootfs.open(&file.filename, OpenFlags::NONE).unwrap(),
                )
                .expect("can't link file to tmpfs");
        }

        FileItem::fs_open("/var", OpenFlags::O_DIRECTORY)
            .expect("can't open /var")
            .mkdir("tmp")
            .expect("can't create tmp dir");

        // Initialize the Dentry node.
        // dentry::dentry_init(rootfs);
        // FileItem::fs_open("/bin", OpenFlags::O_DIRECTORY)
        //     .expect("can't open /bin")
        //     .link(
        //         "sleep",
        //         FileItem::fs_open("busybox", OpenFlags::NONE)
        //             .expect("not hava busybox file")
        //             .inner
        //             .clone(),
        //     )
        //     .expect("can't link busybox to /bin/sleep");
    }

    // enable interrupts
    enable_irq();

    // cache task with task templates
    // cache_task_template("/bin/busybox").expect("can't cache task");
    // cache_task_template("./busybox").expect("can't cache task");
    // cache_task_template("busybox").expect("can't cache task");
    // cache_task_template("./runtest.exe").expect("can't cache task");
    // cache_task_template("entry-static.exe").expect("can't cache task");
    // cache_task_template("libc.so").expect("can't cache task");
    // cache_task_template("lmbench_all").expect("can't cache task");

    // init kernel threads and async executor
    tasks::init();

    println!("Task All Finished!");
}
