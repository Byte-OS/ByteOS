use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use alloc::boxed::Box;
use arch::{get_time_ms, trap_pre_handle, user_restore, ContextOps};
use executor::{current_task, yield_now, AsyncTask, Executor, KernelTask};
use log::debug;

use crate::syscall::syscall;

use self::initproc::initproc;

mod initproc;

pub struct NextTick(usize);

impl Future for NextTick {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let curr = get_time_ms();
        if curr < self.0 {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

#[no_mangle]
// for avoiding the rust cycle check. user extern and nomangle
pub fn user_entry() -> Box<dyn Future<Output = ()> + Send + Sync> {
    Box::new(async { user_entry_inner().await })
}

pub async fn user_entry_inner() {
    let task = current_task().as_user_task().unwrap();
    debug!("user_entry, task: {}", task.get_task_id());
    let cx_ref = unsafe { task.get_cx_ptr().as_mut().unwrap() };

    cx_ref.set_sepc(0x1000);
    cx_ref.set_sp(0x7fff_fff8);

    loop {
        if let Some(exit_code) = task.exit_code() {
            debug!("program exit with code: {}", exit_code);
            break;
        }
        unsafe {
            user_restore(cx_ref);
        }
        let trap_type = trap_pre_handle(cx_ref);
        match trap_type {
            arch::TrapType::Breakpoint => {}
            arch::TrapType::UserEnvCall => {
                info!("user env call: {}", cx_ref.syscall_number());
                // if syscall ok
                let args = cx_ref.args();
                let args = [
                    args[0], args[1], args[2], args[3], args[4], args[5], args[6],
                ];
                let call_number = cx_ref.syscall_number();
                let result = syscall(call_number, args)
                    .await
                    .map_or_else(|e| -e.code(), |x| x as isize)
                    as usize;
                cx_ref.set_ret(result);
                cx_ref.syscall_ok();
            }
            arch::TrapType::Time => {
                info!("time interrupt from user");
            }
            arch::TrapType::Unknown => {
                debug!("unknown trap: {:#x?}", cx_ref);
                panic!("");
            }
        }
        yield_now().await;
    }
}

pub fn init() {
    let mut exec = Executor::new();
    exec.spawn(KernelTask::new(initproc()));
    // exec.spawn()
    exec.run();
}
