use alloc::{sync::Arc, vec::Vec};
use devices::get_net_device;
use executor::{
    current_task, thread, yield_now, AsyncTask, Executor, KernelTask, TaskId, UserTask, TASK_QUEUE,
};

use crate::syscall::{exec_with_process, NET_SERVER};
use crate::user::entry::user_entry;

use self::initproc::initproc;

mod async_ops;
pub mod elf;
mod initproc;

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
        if TASK_QUEUE.lock().len() == 0 {
            break;
        }
        let res = get_net_device(0).recv(&mut buffer);
        if let Ok(rlen) = res {
            NET_SERVER.analysis_net_data(&buffer[..rlen]);
        }
        yield_now().await;
    }
}

pub fn init() {
    let mut exec = Executor::new();
    exec.spawn(KernelTask::new(initproc()));
    #[cfg(feature = "net")]
    exec.spawn(KernelTask::new(handle_net()));
    // exec.spawn()
    exec.run();
}

pub async fn add_user_task(filename: &str, args: Vec<&str>, envp: Vec<&str>) -> TaskId {
    let curr_task = current_task();
    let task = UserTask::new(
        user_entry(),
        Arc::downgrade(&current_task()),
        initproc::USER_WORK_DIR,
    );

    task.before_run();
    exec_with_process(task.clone(), filename, args, envp)
        .await
        .expect("can't add task to excutor");
    thread::spawn(task.clone());
    curr_task.before_run();

    task.get_task_id()
}
