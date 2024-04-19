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
mod drivers;

#[macro_use]
extern crate logging;
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate log;

mod epoll;
// mod modules;
mod panic;
mod socket;
mod syscall;
mod tasks;
mod user;

use arch::addr::{PhysPage, VirtPage};
use arch::api::ArchInterface;
use arch::{disable_irq, enable_irq, TrapFrame, TrapFrameArgs, TrapType, VIRT_ADDR_START};
use devices::{self, get_int_device};
use executor::current_task;
use fdt::node::FdtNode;
use frame_allocator::{self, frame_alloc_persist, frame_unalloc};
use hal;
use tasks::UserTask;
use user::user_cow_int;
use vfscore::OpenFlags;

use crate::tasks::{current_user_task, FileItem};
use crate::user::task_ilegal;

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

    fn init_allocator() {
        allocator::init();
    }

    /// Handle kernel interrupt
    fn kernel_interrupt(cx_ref: &mut TrapFrame, trap_type: TrapType) {
        match trap_type {
            TrapType::StorePageFault(addr)
            | TrapType::InstructionPageFault(addr)
            | TrapType::LoadPageFault(addr) => {
                if addr > VIRT_ADDR_START {
                    panic!("kernel error: {:#x}", addr);
                }
                // judge whether it is trigger by a user_task handler.
                if let Some(task) = current_task().as_any().downcast::<UserTask>().ok() {
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
            TrapType::IllegalInstruction(addr) => {
                if addr > VIRT_ADDR_START {
                    return;
                }
                let task = current_user_task();
                let vpn = VirtPage::from_addr(addr);
                warn!(
                    "store/instruction page fault @ {:#x} vpn: {} ppn: {:?}",
                    addr,
                    vpn,
                    task.page_table.translate(addr.into()),
                );
                warn!("the fault occurs @ {:#x}", cx_ref[TrapFrameArgs::SEPC]);
                // warn!("user_task map: {:#x?}", task.pcb.lock().memset);
                warn!(
                    "mapped ppn addr: {:#x} @ {:?}",
                    cx_ref[TrapFrameArgs::SEPC],
                    task.page_table
                        .translate(cx_ref[TrapFrameArgs::SEPC].into())
                );
                task_ilegal(&task, cx_ref[TrapFrameArgs::SEPC], cx_ref);
                // panic!("illegal Instruction")
                // let signal = task.tcb.read().signal.clone();
                // if signal.has_sig(SignalFlags::SIGSEGV) {
                //     task.exit_with_signal(SignalFlags::SIGSEGV.num());
                // } else {
                //     return UserTaskControlFlow::Break
                // }
                // current_user_task()
                //     .tcb
                //     .write()
                //     .signal
                //     .add_signal(SignalFlags::SIGSEGV);
                // return UserTaskControlFlow::Break;
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
        disable_irq();
        if hart_id == 0 {
            extern "C" {
                fn start();
                fn end();
            }

            println!("HEAP_SIZE: {:#x}", allocator::HEAP_SIZE);

            println!("run kernel @ hart {}", hart_id);

            info!("program size: {}KB", (end as usize - start as usize) / 1024);

            // initialize interrupt
            hal::interrupt::init();

            // get devices and init
            devices::regist_devices_irq();

            // initialize filesystem
            fs::init();
            {
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

            // loop {
            //     info!("3");
            // }

            // init kernel threads and async executor
            tasks::init();

            println!("Task All Finished!");
        } else {
            println!("run kernel @ hart {}", hart_id);

            // initialize interrupt
            hal::interrupt::init();

            // enable_irq();

            loop {
                info!("aux core");
            }
        }
    }
}
