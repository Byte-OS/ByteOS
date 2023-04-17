use core::{cmp::min, future::Future, mem::size_of, ops::Add, pin::Pin};

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use arch::{
    paddr_c, ppn_c, Context, ContextOps, PTEFlags, PageTable, PhysPage, VirtAddr, VirtPage,
    PAGE_SIZE, PTE,
};
use devfs::{Stdin, Stdout};
use frame_allocator::{frame_alloc, frame_alloc_much, FrameTracker};
use fs::{File, SeekFrom};
use log::debug;
use sync::{Mutex, MutexGuard};

use crate::{task_id_alloc, thread, AsyncTask, TaskId, FUTURE_LIST};

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

#[derive(Clone)]
pub enum MemType {
    CodeSection,
    Stack,
    Mmap,
    Shared(Option<File>, usize, usize), // file, start, len
    Clone,
    PTE,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct MemTrack {
    pub mem_type: MemType,
    pub vpn: VirtPage,
    pub tracker: Arc<FrameTracker>,
}

impl Drop for MemTrack {
    fn drop(&mut self) {
        match &self.mem_type {
            MemType::Shared(file, start, len) => {
                file.as_ref().map(|file| match Arc::strong_count(file) > 1 {
                    true => {}
                    false => {
                        let offset = self.vpn.to_addr() - start;
                        let wlen = min(len - offset, PAGE_SIZE);

                        let bytes = unsafe {
                            core::slice::from_raw_parts_mut(
                                ppn_c(self.tracker.0).to_addr() as *mut u8,
                                wlen,
                            )
                        };
                        file.seek(SeekFrom::SET(offset))
                            .expect("can't write data to file");
                        file.write(bytes).expect("can't write data to file at drop");
                    }
                });
            }
            _ => {}
        }
    }
}

pub struct TaskInner {
    pub memset: Vec<MemTrack>,
    pub cx: Context,
    pub fd_table: FileTable,
    pub exit_code: Option<usize>,
    pub curr_dir: String,
    pub heap: usize,
    pub entry: usize,
    pub children: Vec<Arc<UserTask>>,
}

#[allow(dead_code)]
pub struct UserTask {
    pub task_id: TaskId,
    pub page_table: PageTable,
    pub inner: Mutex<TaskInner>,
    pub parent: Option<Arc<dyn AsyncTask>>,
}

impl Drop for UserTask {
    fn drop(&mut self) {
        FUTURE_LIST.lock().remove(&self.task_id);
    }
}

impl UserTask {
    pub fn new(
        future: Box<dyn Future<Output = ()> + Sync + Send + 'static>,
        parent: Option<Arc<dyn AsyncTask>>,
    ) -> Arc<Self> {
        let ppn = Arc::new(frame_alloc().unwrap());
        let page_table = PageTable::from_ppn(ppn.0);
        let task_id = task_id_alloc();
        let mut memset = Vec::new();
        memset.push(MemTrack {
            mem_type: MemType::PTE,
            vpn: VirtPage::new(0),
            tracker: ppn,
        });

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
            children: Vec::new(),
            entry: 0,
        };

        Arc::new(Self {
            page_table,
            task_id,
            parent,
            inner: Mutex::new(inner),
        })
    }

    pub fn inner_map<T>(&self, mut f: impl FnMut(MutexGuard<TaskInner>) -> T) -> T {
        f(self.inner.lock())
    }

    pub fn map(&self, ppn: PhysPage, vpn: VirtPage, flags: PTEFlags) {
        self.page_table.map(ppn, vpn, flags, || {
            self.frame_alloc(VirtPage::new(0), MemType::PTE)
        });
    }

    #[inline]
    pub fn frame_alloc(&self, vpn: VirtPage, mtype: MemType) -> PhysPage {
        let tracker = Arc::new(frame_alloc().expect("can't alloc frame in user_task"));
        let ppn = tracker.0.clone();
        let mut inner = self.inner.lock();

        // check vpn exists, replace it if exists.
        let finded = inner
            .memset
            .iter_mut()
            .find(|x| x.vpn.to_addr() != 0 && x.vpn == vpn);

        match finded {
            Some(f_tracker) => f_tracker.tracker = tracker.clone(),
            None => {
                let mem_tracker = MemTrack {
                    mem_type: mtype,
                    vpn,
                    tracker,
                };
                inner.memset.push(mem_tracker);
            }
        }
        drop(inner);
        if vpn.to_addr() != 0 {
            // TODO: set flags by MemType.
            self.map(ppn, vpn, PTEFlags::UVRWX);
        }
        ppn
    }

    pub fn frame_alloc_much(&self, vpn: VirtPage, mtype: MemType, count: usize) -> PhysPage {
        assert!(count > 0, "can't alloc count = 0 in user_task frame_alloc");
        let mut trackers: Vec<_> = frame_alloc_much(count)
            .expect("can't alloc frame in user_task")
            .into_iter()
            .enumerate()
            .map(|(i, x)| {
                let vpn = match vpn.to_addr() == 0 {
                    true => vpn,
                    false => vpn.add(i),
                };
                MemTrack {
                    mem_type: mtype.clone(),
                    vpn,
                    tracker: Arc::new(x),
                }
            })
            .collect();
        // add tracker and map memory.
        trackers.iter().for_each(|target| {
            let mut inner = self.inner.lock();
            let finded = inner
                .memset
                .iter_mut()
                .find(|x| x.vpn.to_addr() != 0 && x.vpn == target.vpn);

            match finded {
                Some(f_tracker) => {
                    f_tracker.tracker = target.tracker.clone();
                }
                None => {
                    inner.memset.push(target.clone());
                }
            }
            drop(inner);
            if vpn.to_addr() != 0 {
                // TODO: set flags by MemType.
                self.map(target.tracker.0, target.vpn, PTEFlags::UVRWX);
            }
        });
        let ppn = trackers[0].tracker.0.clone();
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
                self.frame_alloc(VirtPage::new(i + 1), MemType::CodeSection);
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
    pub fn fork(
        self: Arc<Self>,
        future: Box<dyn Future<Output = ()> + Sync + Send + 'static>,
    ) -> Arc<Self> {
        // Give the frame_tracker in the memset a type.
        // it will contains the frame used for page mapping、
        // mmap or text section.
        // and then we can implement COW(copy on write).
        let new_task = Self::new(future, Some(self.clone()));
        // clone fd_table
        new_task.inner.lock().fd_table.0 = self.inner.lock().fd_table.0.clone();

        new_task.inner.lock().cx.clone_from(&self.inner.lock().cx);
        new_task.inner.lock().cx.set_ret(0);
        self.inner.lock().children.push(new_task.clone());
        self.inner
            .lock()
            .memset
            .iter()
            .for_each(|x| match &x.mem_type {
                MemType::CodeSection | MemType::Stack => {
                    new_task.inner.lock().memset.push(MemTrack {
                        mem_type: MemType::Clone,
                        vpn: x.vpn,
                        tracker: x.tracker.clone(),
                    });
                    new_task.map(
                        x.tracker.0,
                        x.vpn,
                        PTEFlags::U | PTEFlags::V | PTEFlags::R | PTEFlags::X,
                    );
                }
                MemType::Shared(file, start, len) => {
                    new_task.inner.lock().memset.push(MemTrack {
                        mem_type: MemType::Shared(file.clone(), *start, *len),
                        vpn: x.vpn,
                        tracker: x.tracker.clone(),
                    });
                    new_task.map(
                        x.tracker.0,
                        x.vpn,
                        PTEFlags::U | PTEFlags::V | PTEFlags::R | PTEFlags::X,
                    );
                }
                _ => {}
            });

        thread::spawn(new_task.clone());
        new_task
    }

    pub fn push_str(&self, str: &str) -> usize {
        const ULEN: usize = size_of::<usize>();
        let mut inner = self.inner.lock();
        let bytes = str.as_bytes();
        let len = bytes.len();
        let sp = inner.cx.sp() - (len + ULEN) / ULEN * ULEN;

        let phys_sp = paddr_c(self.page_table.virt_to_phys(VirtAddr::new(sp)));
        unsafe {
            core::slice::from_raw_parts_mut(phys_sp.addr() as *mut u8, len).copy_from_slice(bytes);
        }
        inner.cx.set_sp(sp);
        sp
    }

    pub fn push_num(&self, num: usize) -> usize {
        const ULEN: usize = size_of::<usize>();
        let mut inner = self.inner.lock();
        let sp = inner.cx.sp() - ULEN;

        let phys_sp = paddr_c(self.page_table.virt_to_phys(VirtAddr::new(sp)));

        unsafe {
            (phys_sp.addr() as *mut usize).write(num);
        }
        inner.cx.set_sp(sp);
        sp
    }

    pub fn get_last_free_addr(&self) -> VirtAddr {
        VirtAddr::new(
            self.inner
                .lock()
                .memset
                .iter()
                .fold(0, |acc, x| match x.vpn.to_addr() > acc {
                    true => x.vpn.to_addr(),
                    false => acc,
                })
                + PAGE_SIZE,
        )
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
