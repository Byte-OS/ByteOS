mod async_ops;
pub mod elf;
pub mod exec;
mod filetable;
mod initproc;
mod memset;
mod shm;
mod signal;
mod task;

use self::initproc::initproc;
use crate::consts::USER_WORK_DIR;
use alloc::{
    string::String,
    sync::Weak,
    {sync::Arc, vec::Vec},
};
pub use async_ops::futex_wake;
use exec::exec_with_process;
use executor::{current_task, thread, AsyncTask, TaskId, DEFAULT_EXECUTOR};
use fs::pathbuf::PathBuf;
pub use memset::MemType;
pub use signal::SignalList;
pub use task::UserTask;

pub enum UserTaskControlFlow {
    Continue,
    Break,
}

pub fn init() {
    DEFAULT_EXECUTOR.init();
    thread::spawn_blank(initproc());
    // #[cfg(feature = "net")]
    // thread::spawn_blank(KernelTask::new(handle_net()));
}

pub fn run_tasks() {
    DEFAULT_EXECUTOR.run()
}

pub async fn add_user_task(filename: &str, args: Vec<&str>, envp: Vec<&str>) -> TaskId {
    let curr_task = current_task();
    let task = UserTask::new(Weak::new(), USER_WORK_DIR);
    task.before_run();
    exec_with_process(
        task.clone(),
        PathBuf::new(),
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
