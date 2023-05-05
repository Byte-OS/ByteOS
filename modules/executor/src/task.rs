use core::{
    cmp::min,
    future::Future,
    mem::size_of,
    ops::{Add, Deref, DerefMut},
    pin::Pin,
};

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use arch::{
    paddr_c, ppn_c, Context, ContextOps, PTEFlags, PageTable, PhysPage, VirtAddr, VirtPage,
    PAGE_SIZE, PTE,
};
use devfs::{Stdin, Stdout};
use frame_allocator::{ceil_div, frame_alloc, frame_alloc_much, FrameTracker};
use fs::{File, SeekFrom};
use log::{debug, warn};
pub use signal::{SigAction, SigProcMask, SignalFlags};
use sync::{Mutex, MutexGuard};

use crate::{signal::SignalList, task_id_alloc, thread, AsyncTask, TaskId, FUTURE_LIST, TMS};

pub type FutexTable = BTreeMap<usize, Vec<usize>>;

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
        arr[0x101] = PTE::from_addr(0x4000_0000, PTEFlags::GVRWX);
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

// pub struct FileTable(pub [Option<File>; FILE_MAX]);

#[derive(Clone)]
pub struct FileTable(pub Vec<Option<File>>);

impl FileTable {
    pub fn new() -> Self {
        let mut file_table: Vec<Option<File>> = vec![FD_NONE; FILE_MAX];
        // let mut file_table = [FD_NONE; FILE_MAX];
        file_table[0] = Some(Arc::new(Stdin::new()));
        file_table[1] = Some(Arc::new(Stdout));
        file_table[2] = Some(Arc::new(Stdout));
        // file_table.push(Some(Arc::new(Stdin)));
        // file_table.push(Some(Arc::new(Stdout)));
        // file_table.push(Some(Arc::new(Stdout)));

        Self(file_table)
    }
}

impl Deref for FileTable {
    type Target = Vec<Option<File>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FileTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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

fn rlimits_new() -> Vec<usize> {
    let mut rlimits = vec![0usize; 8];
    rlimits[7] = FILE_MAX;
    rlimits
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
    pub tms: TMS,
    pub rlimits: Vec<usize>,
    pub signal: SignalList,
    pub sigmask: SigProcMask,
    pub sigaction: Arc<Mutex<[SigAction; 64]>>,
    pub set_child_tid: usize,
    pub futex_table: Arc<Mutex<FutexTable>>,
    pub clear_child_tid: usize,
}

#[allow(dead_code)]
pub struct UserTask {
    pub task_id: TaskId,
    pub page_table: PageTable,
    pub inner: Mutex<TaskInner>,
    pub parent: Weak<dyn AsyncTask>,
}

impl Drop for UserTask {
    fn drop(&mut self) {
        warn!("drop user task: {}", self.task_id);
        FUTURE_LIST.lock().remove(&self.task_id);
    }
}

impl UserTask {
    pub fn new(
        future: Box<dyn Future<Output = ()> + Sync + Send + 'static>,
        parent: Weak<dyn AsyncTask>,
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
        arr[0x101] = PTE::from_addr(0x4000_0000, PTEFlags::GVRWX);
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
            tms: Default::default(),
            rlimits: rlimits_new(),
            sigmask: SigProcMask::new(),
            sigaction: Arc::new(Mutex::new([SigAction::new(); 64])),
            set_child_tid: 0,
            clear_child_tid: 0,
            signal: SignalList::new(),
            futex_table: Arc::new(Mutex::new(BTreeMap::new())),
        };

        Arc::new(Self {
            page_table,
            task_id,
            parent,
            inner: Mutex::new(inner),
        })
    }

    pub fn inner_map<T>(&self, mut f: impl FnMut(&mut MutexGuard<TaskInner>) -> T) -> T {
        f(&mut self.inner.lock())
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
        let inner = self.inner.lock();
        let curr_page = inner.heap / PAGE_SIZE;
        let after_page = (inner.heap as isize + incre) as usize / PAGE_SIZE;
        drop(inner);
        // need alloc frame page
        if after_page > curr_page {
            for i in curr_page..after_page {
                self.frame_alloc(VirtPage::new(i + 1), MemType::CodeSection);
            }
        }
        let mut inner = self.inner.lock();
        inner.heap = (inner.heap as isize + incre) as usize;
        inner.heap
    }

    pub fn heap(&self) -> usize {
        self.inner.lock().heap
    }

    #[inline]
    pub fn exit(&self, exit_code: usize) {
        let uaddr = self.inner.lock().clear_child_tid;
        if uaddr != 0 {
            extern "Rust" {
                pub fn futex_wake(
                    task: Arc<Mutex<FutexTable>>,
                    uaddr: usize,
                    wake_count: usize,
                ) -> usize;
            }
            debug!("write addr: {:#x}", uaddr);
            let addr = self.page_table.virt_to_phys(VirtAddr::from(uaddr));
            unsafe {
                (paddr_c(addr).addr() as *mut u32).write(0);
                futex_wake(self.inner.lock().futex_table.clone(), uaddr, 1);
            }
        }
        self.inner.lock().exit_code = Some(exit_code);
        FUTURE_LIST.lock().remove(&self.task_id);
        // recycle memory resouces
        self.inner.lock().memset.clear();
        self.parent.upgrade().map(|x| {
            x.clone()
                .as_user_task()
                .map(|x| x.inner.lock().signal.add_signal(SignalFlags::SIGCHLD))
        });
    }

    #[inline]
    pub fn exit_with_signal(&self, signal: usize) {
        self.exit(128 + signal);
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
        let parent_task: Arc<dyn AsyncTask> = self.clone();
        let new_task = Self::new(future, Arc::downgrade(&parent_task));
        // clone fd_table
        new_task.inner.lock().fd_table.0 = self.inner.lock().fd_table.0.clone();

        new_task.inner.lock().cx.clone_from(&self.inner.lock().cx);
        new_task.inner.lock().cx.set_ret(0);
        new_task.inner_map(|inner| {
            inner.curr_dir = self.inner.lock().curr_dir.clone();
        });
        self.inner.lock().children.push(new_task.clone());
        self.inner
            .lock()
            .memset
            .iter()
            .for_each(|x| match &x.mem_type {
                // 后面再考虑 copy on write 的问题.
                MemType::Stack | MemType::CodeSection | MemType::Clone => {
                    new_task
                        .frame_alloc(x.vpn, x.mem_type.clone())
                        .copy_value_from_another(x.tracker.0);
                }
                // MemType::CodeSection | MemType::Clone => {
                //     new_task.inner.lock().memset.push(MemTrack {
                //         mem_type: MemType::Clone,
                //         vpn: x.vpn,
                //         tracker: x.tracker.clone(),
                //     });
                //     new_task.map(
                //         x.tracker.0,
                //         x.vpn,
                //         PTEFlags::U | PTEFlags::V | PTEFlags::R | PTEFlags::X,
                //     );
                // }
                MemType::Shared(file, start, len) => {
                    new_task.inner.lock().memset.push(MemTrack {
                        mem_type: MemType::Shared(file.clone(), *start, *len),
                        vpn: x.vpn,
                        tracker: x.tracker.clone(),
                    });
                    new_task.map(
                        x.tracker.0,
                        x.vpn,
                        PTEFlags::U | PTEFlags::V | PTEFlags::R | PTEFlags::X | PTEFlags::W,
                    );
                }
                _ => {}
            });

        thread::spawn(new_task.clone());
        new_task
    }

    #[inline]
    pub fn thread_clone(
        self: Arc<Self>,
        future: Box<dyn Future<Output = ()> + Sync + Send + 'static>,
    ) -> Arc<Self> {
        // Give the frame_tracker in the memset a type.
        // it will contains the frame used for page mapping、
        // mmap or text section.
        // and then we can implement COW(copy on write).
        let parent_task: Arc<dyn AsyncTask> = self.clone();

        let task_id = task_id_alloc();
        let mut inner = self.inner.lock();
        let mut new_inner = TaskInner {
            memset: inner.memset.clone(),
            cx: inner.cx.clone(),
            fd_table: inner.fd_table.clone(),
            exit_code: None,
            curr_dir: inner.curr_dir.clone(),
            heap: inner.heap,
            children: inner.children.clone(),
            entry: 0,
            tms: Default::default(),
            rlimits: rlimits_new(),
            sigmask: inner.sigmask.clone(),
            sigaction: inner.sigaction.clone(),
            set_child_tid: 0,
            futex_table: inner.futex_table.clone(),
            clear_child_tid: 0,
            signal: SignalList::new(),
        };

        new_inner.cx.set_ret(0);

        let new_task = Arc::new(Self {
            page_table: self.page_table.clone(),
            task_id,
            parent: Arc::downgrade(&parent_task),
            inner: Mutex::new(new_inner),
        });
        inner.children.push(new_task.clone());

        FUTURE_LIST.lock().insert(task_id, Pin::from(future));

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

    pub fn push_arr(&self, buffer: &[u8]) -> usize {
        const ULEN: usize = size_of::<usize>();
        let mut inner = self.inner.lock();
        let len = buffer.len();
        let sp = inner.cx.sp() - ceil_div(len, ULEN) * ULEN;

        let phys_sp = paddr_c(self.page_table.virt_to_phys(VirtAddr::new(sp)));
        unsafe {
            core::slice::from_raw_parts_mut(phys_sp.addr() as *mut u8, len).copy_from_slice(buffer);
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
                .filter(|x| match x.mem_type {
                    MemType::Stack => false,
                    _ => true,
                })
                .fold(0, |acc, x| match x.vpn.to_addr() > acc {
                    true => x.vpn.to_addr(),
                    false => acc,
                })
                + PAGE_SIZE,
        )
    }

    pub fn get_fd(&self, index: usize) -> Option<File> {
        let inner = self.inner.lock();
        match index >= inner.rlimits[7] {
            true => None,
            false => inner.fd_table.0[index].clone(),
        }
    }

    pub fn set_fd(&self, index: usize, value: Option<File>) {
        let mut inner = self.inner.lock();
        match index >= inner.rlimits[7] {
            true => {}
            false => inner.fd_table.0[index] = value,
        }
    }

    pub fn alloc_fd(&self) -> Option<usize> {
        let mut inner = self.inner.lock();
        let index = inner
            .fd_table
            .0
            .iter()
            .enumerate()
            .find(|(i, x)| x.is_none() && *i < inner.rlimits[7])
            .map(|(i, _)| i);
        if index.is_none() && inner.fd_table.0.len() < inner.rlimits[7] {
            inner.fd_table.0.push(None);
            Some(inner.fd_table.0.len() - 1)
        } else {
            index
        }
    }

    pub fn used_fd(&self) -> usize {
        self.inner
            .lock()
            .fd_table
            .0
            .iter()
            .filter(|x| x.is_some())
            .count()
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
