#![no_main]
#![no_std]
#![feature(extract_if)]
#![feature(let_chains)]
#![feature(used_with_arg)]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate log;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate srv_iface;

mod consts;
// mod socket;
mod syscall;
mod tasks;
// mod user;
mod utils;

use alloc::sync::Arc;
use devices::{driver_define, ipc_uart::IPCUart};
use fs::file::File;
use vfscore::OpenFlags;

sel4_runtime::entry_point!(main);

driver_define!({ Some(Arc::new(IPCUart)) });

/// The kernel entry
fn main() -> ! {
    sel4_runtime::init_log!(log::LevelFilter::Debug);
    // Ensure this is the first core
    runtime::init();

    let str = include_str!("banner.txt");
    print!("{}", str);

    println!("run kernel @ sel4");

    devices::prepare_drivers();

    // initialize filesystem
    fs::init();
    {
        File::open("/var".into(), OpenFlags::O_DIRECTORY)
            .expect("can't open /var")
            .mkdir("tmp")
            .expect("can't create tmp dir");
    }

    let file = File::open("/".into(), OpenFlags::O_DIRECTORY).unwrap();
    for files in file.read_dir().unwrap() {
        println!("file entry: {}", files.filename);
    }
    // init kernel threads and async executor
    tasks::init();
    println!("run tasks");
    // tasks::run_tasks();

    println!("Task All Finished!");
    loop {}
}
