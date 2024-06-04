use alloc::string::String;
use alloc::sync::Weak;
use alloc::{sync::Arc, vec::Vec};
use devices::get_net_device;
use executor::{current_task, thread, yield_now, AsyncTask, TaskId, DEFAULT_EXECUTOR};
use hal::{ITimerVal, TimeVal};
use polyhal::get_cpu_num;

use crate::syscall::{exec_with_process, NET_SERVER};
use crate::user::entry::user_entry;

use self::initproc::initproc;

mod async_ops;
pub mod elf;
mod filetable;
mod initproc;
mod memset;
mod shm;
mod signal;
mod task;

pub use filetable::FileItem;
pub use memset::{MapTrack, MemArea, MemType};
pub use shm::{MapedSharedMemory, SharedMemory, SHARED_MEMORY};
pub use signal::SignalList;
pub use task::UserTask;

pub use async_ops::{
    futex_requeue, futex_wake, WaitFutex, WaitHandleAbleSignal, WaitPid, WaitSignal,
};

pub enum UserTaskControlFlow {
    Continue,
    Break,
}

pub fn hexdump(data: &[u8], mut start_addr: usize) {
    const PRELAND_WIDTH: usize = 70;
    logging::println!("{:-^1$}", " hexdump ", PRELAND_WIDTH);
    for offset in (0..data.len()).step_by(16) {
        logging::print!("{:08x} ", start_addr);
        start_addr += 0x10;
        for i in 0..16 {
            if offset + i < data.len() {
                logging::print!("{:02x} ", data[offset + i]);
            } else {
                logging::print!("{:02} ", "");
            }
        }

        logging::print!("{:>6}", ' ');

        for i in 0..16 {
            if offset + i < data.len() {
                let c = data[offset + i];
                if c >= 0x20 && c <= 0x7e {
                    logging::print!("{}", c as char);
                } else {
                    logging::print!(".");
                }
            } else {
                logging::print!("{:02} ", "");
            }
        }

        logging::println!("");
    }
    logging::println!("{:-^1$}", " hexdump end ", PRELAND_WIDTH);
}

#[allow(dead_code)]
pub async fn handle_net() {
    let mut buffer = vec![0u8; 2048];
    // #[cfg(feature = "net")]
    loop {
        let res = get_net_device(0).recv(&mut buffer);
        if let Ok(rlen) = res {
            NET_SERVER.analysis_net_data(&buffer[..rlen]);
        }
        yield_now().await;
    }
}

pub fn init() {
    DEFAULT_EXECUTOR.init(get_cpu_num());
    thread::spawn_blank(initproc());
    #[cfg(feature = "net")]
    thread::spawn_blank(KernelTask::new(handle_net()));
}

pub fn run_tasks() {
    DEFAULT_EXECUTOR.run()
}

pub async fn add_user_task(filename: &str, args: Vec<&str>, envp: Vec<&str>) -> TaskId {
    let curr_task = current_task();
    let task = UserTask::new(Weak::new(), initproc::USER_WORK_DIR);
    task.before_run();
    exec_with_process(
        task.clone(),
        String::from(filename),
        args.into_iter().map(String::from).collect(),
        envp.into_iter().map(String::from).collect(),
    )
    .await
    .expect("can't add task to excutor");
    curr_task.before_run();
    thread::spawn(task.clone(), user_entry());

    task.get_task_id()
}

#[inline]
pub fn current_user_task() -> Arc<UserTask> {
    current_task().downcast_arc::<UserTask>().ok().unwrap()
}

// tms_utime记录的是进程执行用户代码的时间.
// tms_stime记录的是进程执行内核代码的时间.
// tms_cutime记录的是子进程执行用户代码的时间.
// tms_ustime记录的是子进程执行内核代码的时间.
#[allow(dead_code)]
#[derive(Default, Clone, Copy)]
pub struct TMS {
    pub utime: u64,
    pub stime: u64,
    pub cutime: u64,
    pub cstime: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessTimer {
    pub timer: ITimerVal,
    pub next: TimeVal,
    pub last: TimeVal,
}
