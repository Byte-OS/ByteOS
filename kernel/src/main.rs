#![no_main]
#![no_std]
#![feature(extract_if)]
#![feature(async_closure)]
#![feature(let_chains)]

// include modules drivers
// mod drivers;
include!(concat!(env!("OUT_DIR"), "/drivers.rs"));

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate log;
#[macro_use]
extern crate polyhal;
#[macro_use]
extern crate cfg_if;

extern crate polyhal_boot;
extern crate polyhal_trap;

#[macro_use]
mod logging;

mod consts;
mod panic;
mod socket;
mod syscall;
mod tasks;
mod user;
mod utils;

use crate::tasks::{current_user_task, File};
use crate::user::task_ilegal;
use core::hint::spin_loop;
use devices::{self, get_int_device, PAGE_SIZE, VIRT_ADDR_START};
use executor::current_task;
use polyhal::common::PageAlloc;
use polyhal::irq::IRQ;
use polyhal::mem::{get_fdt, get_mem_areas};
use polyhal::{va, PhysAddr};
use polyhal_trap::trap::TrapType;
use polyhal_trap::trapframe::{TrapFrame, TrapFrameArgs};
use runtime::frame::{frame_alloc_persist, frame_unalloc};
use tasks::UserTask;
use user::user_cow_int;
use vfscore::OpenFlags;

pub struct PageAllocImpl;

impl PageAlloc for PageAllocImpl {
    #[inline]
    fn alloc(&self) -> PhysAddr {
        unsafe { frame_alloc_persist().expect("can't alloc frame") }
    }

    #[inline]
    fn dealloc(&self, paddr: PhysAddr) {
        unsafe {
            frame_unalloc(paddr);
            paddr.clear_len(PAGE_SIZE);
        }
    }
}

#[export_name = "_interrupt_for_arch"]
/// Handle kernel interrupt
fn kernel_interrupt(cx_ref: &mut TrapFrame, trap_type: TrapType) {
    match trap_type {
        TrapType::StorePageFault(addr)
        | TrapType::InstructionPageFault(addr)
        | TrapType::LoadPageFault(addr) => {
            if addr > VIRT_ADDR_START {
                panic!(
                    "kernel page error: {:#x} sepc: {:#x}",
                    addr,
                    cx_ref[TrapFrameArgs::SEPC]
                );
            }
            // judge whether it is trigger by a user_task handler.
            if let Some(task) = current_task().downcast_arc::<UserTask>().ok() {
                let cx_ref = task.force_cx_ref();
                if task.pcb.is_locked() {
                    // task.pcb.force_unlock();
                    unsafe {
                        task.pcb.force_unlock();
                    }
                }
                user_cow_int(task, cx_ref, va!(addr));
            } else {
                panic!("page fault: {:#x?}", trap_type);
            }
        }
        TrapType::IllegalInstruction(addr) => {
            if addr > VIRT_ADDR_START {
                return;
            }
            let task = current_user_task();
            warn!(
                "illegal instruction fault @ {:#x} paddr: {:?}",
                addr,
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
            task_ilegal(&task, va!(cx_ref[TrapFrameArgs::SEPC]), cx_ref);
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

/// The kernel entry
fn main(hart_id: usize) {
    IRQ::int_disable();
    // Ensure this is the first core
    runtime::init();

    let str = include_str!("banner.txt");
    println!("{}", str);

    polyhal::common::init(&PageAllocImpl);
    get_mem_areas().cloned().for_each(|(start, size)| {
        info!("memory area: {:#x} - {:#x}", start, start + size);
        runtime::frame::add_frame_map(start, start + size);
    });

    println!("run kernel @ hart {}", hart_id);

    extern "C" {
        fn _start();
        fn _end();
    }
    info!(
        "program size: {}KB",
        (_start as usize - _end as usize) / 1024
    );

    // Boot all application core.
    // polyhal::multicore::MultiCore::boot_all();

    devices::prepare_drivers();

    if let Ok(fdt) = get_fdt() {
        for node in fdt.all_nodes() {
            devices::try_to_add_device(&node);
        }
    }

    // get devices and init
    devices::regist_devices_irq();

    // TODO: test ebreak
    // Instruction::ebreak();

    // initialize filesystem
    fs::init();
    {
        File::fs_open("/var", OpenFlags::O_DIRECTORY)
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
    IRQ::int_enable();

    // cache task with task templates
    tasks::exec::cache_task_template("./busybox").expect("can't cache task");
    tasks::exec::cache_task_template("./runtest.exe").expect("can't cache task");
    tasks::exec::cache_task_template("entry-static.exe").expect("can't cache task");
    // tasks::exec::cache_task_template("libc.so").expect("can't cache task");
    tasks::exec::cache_task_template("./lua").expect("can't cache task");
    // tasks::exec::cache_task_template("lmbench_all").expect("can't cache task");

    // init kernel threads and async executor
    tasks::init();
    log::info!("run tasks");
    // loop { arch::wfi() }
    tasks::run_tasks();

    println!("Task All Finished!");
}

fn secondary(hart_id: usize) {
    println!("run kernel @ hart {}", hart_id);
    // loop { arch::wfi() }
    // tasks::run_tasks();
    loop {
        spin_loop();
    }
}

polyhal_boot::define_entry!(main, secondary);
