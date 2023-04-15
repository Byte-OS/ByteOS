use core::{future::Future, pin::Pin};

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use arch::{Context, PTEFlags, PageTable, PhysPage, VirtPage, PAGE_SIZE, PTE};
use devfs::{Stdin, Stdout};
use frame_allocator::{frame_alloc, frame_alloc_much, FrameTracker};
use fs::File;
use log::debug;
use sync::Mutex;

use crate::{task_id_alloc, AsyncTask, TaskId, FUTURE_LIST};

#[allow(dead_code)]
pub struct KernelTask {
    page_table: PageTable,
    task_id: TaskId,
    memset: Vec<Arc<FrameTracker>>,
}

impl Drop for KernelTask {
    fn drop(&mut self) {
        FUTURE_LIST.lock().remove(&self.task_id);
    }
}

impl KernelTask {
    pub fn new(future: impl Future<Output = ()> + Sync + Send + 'static) -> Arc<Self> {
        let ppn = Arc::new(frame_alloc().unwrap());
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

const FILE_MAX: usize = 255;
const FD_NONE: Option<File> = Option::None;

pub struct FileTable([Option<File>; FILE_MAX]);

impl FileTable {
    pub fn new() -> Self {
        let mut file_table = [FD_NONE; FILE_MAX];
        file_table[0] = Some(Arc::new(Stdin));
        file_table[1] = Some(Arc::new(Stdout));
        file_table[2] = Some(Arc::new(Stdout));
        Self(file_table)
    }

    pub fn get(&self, index: usize) -> Option<File> {
        self.0[index].clone()
    }

    pub fn set(&mut self, index: usize, value: Option<File>) {
        self.0[index] = value;
    }

    pub fn alloc_fd(&self) -> Option<usize> {
        self.0
            .iter()
            .enumerate()
            .find(|(_, x)| x.is_none())
            .map(|(i, _)| i)
    }
}

pub struct TaskInner {
    pub memset: Vec<Arc<FrameTracker>>,
    pub cx: Context,
    pub fd_table: FileTable,
    pub exit_code: Option<usize>,
    pub curr_dir: String,
    pub heap: usize,
}

#[allow(dead_code)]
pub struct UserTask {
    pub task_id: TaskId,
    pub entry: usize,
    pub page_table: PageTable,
    pub inner: Mutex<TaskInner>,
    pub parent: Option<Arc<UserTask>>,
}

impl Drop for UserTask {
    fn drop(&mut self) {
        FUTURE_LIST.lock().remove(&self.task_id);
    }
}

impl UserTask {
    pub fn new(
        future: Box<dyn Future<Output = ()> + Sync + Send + 'static>,
        parent: Option<Arc<UserTask>>,
    ) -> Arc<Self> {
        let ppn = Arc::new(frame_alloc().unwrap());
        let page_table = PageTable::from_ppn(ppn.0);
        let task_id = task_id_alloc();
        let mut memset = Vec::new();
        memset.push(ppn);

        let arr = page_table.get_pte_list();
        arr[0x100] = PTE::from_addr(0x0000_0000, PTEFlags::GVRWX);
        arr[0x102] = PTE::from_addr(0x8000_0000, PTEFlags::GVRWX);

        FUTURE_LIST.lock().insert(task_id, Pin::from(future));

        let inner = TaskInner {
            memset,
            cx: Context::new(),
            fd_table: FileTable::new(),
            exit_code: None,
            curr_dir: String::from("/"),
            heap: 0,
        };

        Arc::new(Self {
            entry: 0,
            page_table,
            task_id,
            parent,
            inner: Mutex::new(inner),
        })
    }

    pub fn map(&self, ppn: PhysPage, vpn: VirtPage, flags: PTEFlags) {
        self.page_table.map(ppn, vpn, flags, || self.frame_alloc());
    }

    #[inline]
    pub fn frame_alloc(&self) -> PhysPage {
        let tracker = frame_alloc().expect("can't alloc frame in user_task");
        let ppn = tracker.0.clone();
        self.inner.lock().memset.push(Arc::new(tracker));
        ppn
    }

    pub fn frame_alloc_much(&self, count: usize) -> PhysPage {
        assert!(count > 0, "can't alloc count = 0 in user_task frame_alloc");
        let mut trackers: Vec<_> = frame_alloc_much(count)
            .expect("can't alloc frame in user_task")
            .into_iter()
            .map(|x| Arc::new(x))
            .collect();
        let ppn = trackers[0].0.clone();
        self.inner.lock().memset.append(&mut trackers);
        ppn
    }

    pub fn get_cx_ptr(&self) -> *mut Context {
        (&mut self.inner.lock().cx) as *mut Context
    }

    pub fn exit_code(&self) -> Option<usize> {
        self.inner.lock().exit_code
    }

    pub fn sbrk(&self, incre: isize) -> usize {
        let mut inner = self.inner.lock();
        let curr_page = inner.heap / PAGE_SIZE;
        let after_page = (inner.heap as isize + incre) as usize / PAGE_SIZE;
        // need alloc frame page
        if after_page > curr_page {
            for i in curr_page..after_page {
                let ppn = self.frame_alloc();
                self.map(ppn, VirtPage::new(i + 1), PTEFlags::UVRW);
            }
        }
        inner.heap = (inner.heap as isize + incre) as usize;
        inner.heap
    }

    pub fn heap(&self) -> usize {
        self.inner.lock().heap
    }

    #[inline]
    pub fn exit(&self, exit_code: usize) {
        self.inner.lock().exit_code = Some(exit_code);
        FUTURE_LIST.lock().remove(&self.task_id);
    }

    #[inline]
    pub fn fork(&self) -> Arc<Self> {
        // Give the frame_tracker in the memset a type.
        // it will contains the frame used for page mapping、
        // mmap or text section.
        // and then we can implement COW(copy on write).
        todo!("to implement fork in user task.")
    }
}

impl AsyncTask for UserTask {
    fn get_task_id(&self) -> TaskId {
        self.task_id
    }

    fn before_run(&self) {
        self.page_table.change();
    }

    fn as_user_task(self: Arc<Self>) -> Option<Arc<UserTask>> {
        Some(self)
    }
}

pub async fn kernel_entry(future: impl Future<Output = ()> + Sync + Send + 'static) {
    debug!("kernel_entry");
    future.await;
}
