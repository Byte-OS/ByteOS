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
mod user;

use arch::{enable_irq, ArchInterface, Context, PhysPage, TrapType, VIRT_ADDR_START};
use devices::{self, get_int_device};
use executor::{get_current_task, FileItem};
use fdt::node::FdtNode;
use frame_allocator::{self, frame_alloc_persist, frame_unalloc};
use fs::get_filesystem;
use hal;
use user::user_cow_int;
use vfscore::{INodeInterface, OpenFlags};

struct ArchInterfaceImpl;

#[crate_interface::impl_interface]
impl ArchInterface for ArchInterfaceImpl {
    /// handle init_logging
    fn init_logging() {
        let str = include_str!("banner.txt");
        println!("{}", str);
        // initialize logging module
        logging::init(option_env!("LOG"));
    }

    /// Handle kernel interrupt
    fn kernel_interrupt(_ctx: &mut Context, trap_type: TrapType) {
        match trap_type {
            TrapType::StorePageFault(addr) | TrapType::InstructionPageFault(addr) => {
                if addr > VIRT_ADDR_START {
                    panic!("kernel error: {:#x}", addr);
                }
                // judge whether it is trigger by a user_task handler.
                if let Some(task) = get_current_task() {
                    let cx_ref = task.force_cx_ref();
                    if task.pcb.is_locked() {
                        // task.pcb.force_unlock();
                        unsafe {
                            task.pcb.force_unlock();
                        }
                    }
                    user_cow_int(task, cx_ref, addr);
                } else {
                    panic!("page fault: {:?}", trap_type);
                }
            }
            TrapType::SupervisorExternal => {
                get_int_device().try_handle_interrupt(u32::MAX);
            }
            _ => {
                // warn!("trap_type: {:?}  context: {:#x?}", trap_type, cx);
                // debug!("kernel_interrupt");
            }
        };
    }

    /// add memory region
    fn add_memory_region(start: usize, end: usize) {
        frame_allocator::add_frame_map(start, end)
    }

    /// allocate a page
    fn frame_alloc_persist() -> PhysPage {
        unsafe { frame_alloc_persist().expect("can't alloc frame") }
    }

    /// release a page
    fn frame_unalloc(ppn: PhysPage) {
        unsafe { frame_unalloc(ppn) }
        ppn.drop_clear();
    }

    /// prepare drivers
    fn prepare_drivers() {
        devices::prepare_drivers();
    }

    /// try to add a device
    fn try_to_add_device(node: &FdtNode) {
        devices::try_to_add_device(node);
    }

    /// The kernel entry
    fn main(hart_id: usize) {
        // if hart_id != 0 {
        //     loop {}
        // }

        extern "C" {
            fn start();
            fn end();
        }

        println!("run kernel @ hart {}", hart_id);

        info!(
            "program size: {}KB",
            (end as usize - start as usize) / 1024
        );

        // initialize interrupt
        hal::interrupt::init();

        // get devices and init
        devices::regist_devices_irq();

        // initialize filesystem
        fs::init();
        {
            let rootfs = get_filesystem(0).root_dir();
            let tmpfs = FileItem::fs_open("/tmp_home", OpenFlags::O_DIRECTORY)
                .expect("can't open /tmp_home");
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
        // crate::syscall::cache_task_template("/bin/busybox").expect("can't cache task");
        // crate::syscall::cache_task_template("./busybox").expect("can't cache task");
        // crate::syscall::cache_task_template("busybox").expect("can't cache task");
        // crate::syscall::cache_task_template("./runtest.exe").expect("can't cache task");
        // crate::syscall::cache_task_template("entry-static.exe").expect("can't cache task");
        // crate::syscall::cache_task_template("libc.so").expect("can't cache task");
        // crate::syscall::cache_task_template("lmbench_all").expect("can't cache task");

        // init kernel threads and async executor
        tasks::init();

        println!("Task All Finished!");
    }
}
