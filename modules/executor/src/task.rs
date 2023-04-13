use core::future::Future;

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use arch::{PTEFlags, PageTable, PTE};
use frame_allocator::{frame_alloc, FrameTracker};
use log::debug;

use crate::{task_id_alloc, AsyncTask, TaskId, FUTURE_LIST, current_task, yield_now};

#[allow(dead_code)]
pub struct KernelTask {
    page_table: PageTable,
    task_id: TaskId,
    memset: Vec<FrameTracker>,
}

impl Drop for KernelTask {
    fn drop(&mut self) {
        FUTURE_LIST.lock().remove(&self.task_id);
    }
}

impl KernelTask {
    pub fn new(future: impl Future<Output = ()> + Sync + Send + 'static) -> Arc<Self> {
        let ppn = frame_alloc().unwrap();
        let page_table = PageTable::from_ppn(ppn.0);
        let task_id = task_id_alloc();
        let mut memset = Vec::new();
        memset.push(ppn);

        let arr = page_table.get_pte_list();
        arr[0x100] = PTE::from_addr(0x0000_0000, PTEFlags::GVRWX);
        arr[0x102] = PTE::from_addr(0x8000_0000, PTEFlags::GVRWX);

        FUTURE_LIST
            .lock()
            .insert(task_id, Box::pin(kernel_entry(future)));

        Arc::new(Self {
            page_table,
            task_id,
            memset,
        })
    }
}

impl AsyncTask for KernelTask {
    fn get_task_id(&self) -> TaskId {
        self.task_id
    }

    fn before_run(&self) {
        self.page_table.change();
    }
}

#[allow(dead_code)]
pub struct UserTask {
    task_id: TaskId,
    entry: usize,
    memset: Vec<FrameTracker>,
    page_table: PageTable,
}

impl Drop for UserTask {
    fn drop(&mut self) {
        FUTURE_LIST.lock().remove(&self.task_id);
    }
}

impl UserTask {
    pub fn new() -> Arc<Self> {
        let ppn = frame_alloc().unwrap();
        let page_table = PageTable::from_ppn(ppn.0);
        let task_id = task_id_alloc();
        let mut memset = Vec::new();
        memset.push(ppn);

        let arr = page_table.get_pte_list();
        arr[0x100] = PTE::from_addr(0x0000_0000, PTEFlags::GVRWX);
        arr[0x102] = PTE::from_addr(0x8000_0000, PTEFlags::GVRWX);

        FUTURE_LIST
            .lock()
            .insert(task_id, Box::pin(user_entry()));

        Arc::new(Self {
            entry: 0,
            page_table,
            task_id,
            memset,
        })
    }
}

impl AsyncTask for UserTask {
    fn get_task_id(&self) -> TaskId {
        self.task_id
    }

    fn before_run(&self) {
        self.page_table.change();
    }
}

pub async fn kernel_entry(future: impl Future<Output = ()> + Sync + Send + 'static) {
    debug!("kernel_entry");
    future.await;
}

pub async fn user_entry() {
    let task = current_task();
    debug!("user_entry, task: {}", task.get_task_id());
    loop {
        // check close statue
        // if task.is_close() {
        //    break;
        // }
        // run into task, general function, 
        // it will return if it meet interrupt.
        // let context = user_run(task);
        // handle the trap function
        // kill self, will delete self into task_queue.
        // let result = handle_user_trap(task);
        // task.get_context().a0 = result.map_or_else(|e| (-e.code()) as usize, |x| x);
        yield_now().await;
    }
}
