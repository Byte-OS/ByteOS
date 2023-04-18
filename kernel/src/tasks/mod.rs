use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use arch::{get_time, get_time_ms, trap_pre_handle, user_restore, ContextOps, VirtPage};
use executor::{current_task, thread, AsyncTask, Executor, KernelTask, MemType, UserTask};
use log::debug;

use crate::syscall::{exec_with_process, syscall};

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

#[no_mangle]
pub async fn user_entry_inner() {
    loop {
        let task = current_task().as_user_task().unwrap();
        debug!("user_entry, task: {}", task.get_task_id());
        let cx_ref = unsafe { task.get_cx_ptr().as_mut().unwrap() };

        if let Some(exit_code) = task.exit_code() {
            debug!("program exit with code: {}", exit_code);
            break;
        }
        let ustart = 0;
        unsafe {
            user_restore(cx_ref);
        }
        task.inner_map(|mut inner| inner.tms.utime += (get_time() - ustart) as u64);

        let sstart = 0;
        let trap_type = trap_pre_handle(cx_ref);
        match trap_type {
            arch::TrapType::Breakpoint => {}
            arch::TrapType::UserEnvCall => {
                debug!("user env call: {}", cx_ref.syscall_number());
                // if syscall ok
                let args = cx_ref.args();
                let args = [
                    args[0], args[1], args[2], args[3], args[4], args[5], args[6],
                ];
                let call_number = cx_ref.syscall_number();
                cx_ref.syscall_ok();
                let result = syscall(call_number, args)
                    .await
                    .map_or_else(|e| -e.code(), |x| x as isize)
                    as usize;
                cx_ref.set_ret(result);
            }
            arch::TrapType::Time => {
                debug!("time interrupt from user");
            }
            arch::TrapType::Unknown => {
                debug!("unknown trap: {:#x?}", cx_ref);
                panic!("");
            }
            arch::TrapType::StorePageFault(addr) => {
                let vpn = VirtPage::from_addr(addr);
                debug!("store page fault @ {:#x}", addr);
                let mem_tracker = task
                    .inner
                    .lock()
                    .memset
                    .iter()
                    .find(|x| {
                        x.vpn == vpn
                            && match x.mem_type {
                                MemType::Clone => true,
                                _ => false,
                            }
                    })
                    .map(|x| x.tracker.clone());

                match mem_tracker {
                    Some(tracker) => {
                        let src_ppn = tracker.0;
                        let dst_ppn = task.frame_alloc(vpn, MemType::CodeSection);
                        dst_ppn.copy_value_from_another(src_ppn);
                    }
                    None => {
                        // TODO: add stack @ here
                        break;
                    }
                }
            }
        }
        task.inner_map(|mut inner| inner.tms.stime += (get_time() - sstart) as u64);
        // yield_now().await;
    }
}

pub fn init() {
    let mut exec = Executor::new();
    exec.spawn(KernelTask::new(initproc()));
    // exec.spawn()
    exec.run();
}

pub struct WaitPid(pub Arc<UserTask>, pub isize);

impl Future for WaitPid {
    type Output = Arc<UserTask>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = self.0.inner.lock();
        let res = inner.children.iter().find(|x| {
            let inner = x.inner.lock();
            (self.1 == -1 || x.task_id == self.1 as usize) && inner.exit_code.is_some()
        });
        match res {
            Some(task) => Poll::Ready(task.clone()),
            None => Poll::Pending,
        }
    }
}

pub async fn add_user_task(filename: &str, args: Vec<&str>, _envp: Vec<&str>) {
    let task = UserTask::new(user_entry(), Some(current_task()));
    exec_with_process(task.clone(), filename, args)
        .await
        .expect("can't add task to excutor");
    thread::spawn(task.clone());
}
