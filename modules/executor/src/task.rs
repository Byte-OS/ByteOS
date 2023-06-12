use core::{future::Future, mem::size_of, ops::Add, pin::Pin};

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use arch::{
    paddr_c, Context, ContextOps, PTEFlags, PageTable, PhysPage, VirtAddr, VirtPage, PAGE_SIZE,
};
use frame_allocator::{ceil_div, frame_alloc, frame_alloc_much, FrameTracker};
use fs::File;
use log::{debug, warn};
pub use signal::{SigAction, SigProcMask, SignalFlags};
use sync::{Mutex, MutexGuard, RwLock};

use crate::{
    filetable::{rlimits_new, FileTable},
    memset::{MapTrack, MemArea, MemType},
    signal::SignalList,
    task_id_alloc, thread, AsyncTask, TaskId, FUTURE_LIST, TMS,
};

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
        let memset = vec![ppn];

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

pub struct TaskInner {
    pub memset: Vec<MemArea>,
    pub fd_table: FileTable,
    pub curr_dir: String,
    pub heap: usize,
    pub entry: usize,
    pub children: Vec<Arc<UserTask>>,
    pub tms: TMS,
    pub rlimits: Vec<usize>,
    pub sigaction: [SigAction; 64],
    pub futex_table: Arc<Mutex<FutexTable>>,
}

pub struct ThreadControlBlock {
    pub cx: Context,
    pub exit_code: Option<usize>,
    pub sigmask: SigProcMask,
    pub clear_child_tid: usize,
    pub set_child_tid: usize,
    pub signal: SignalList,
}

#[allow(dead_code)]
pub struct UserTask {
    pub task_id: TaskId,
    pub page_table: PageTable,
    pub pcb: Arc<Mutex<TaskInner>>,
    pub parent: Weak<dyn AsyncTask>,
    pub tcb: RwLock<ThreadControlBlock>,
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
        let task_id = task_id_alloc();
        // initialize memset
        let memset = vec![MemArea::new(
            MemType::PTE,
            vec![MapTrack {
                vpn: VirtPage::new(0),
                tracker: ppn.clone(),
            }],
        )];

        FUTURE_LIST.lock().insert(task_id, Pin::from(future));

        let inner = TaskInner {
            memset,
            fd_table: FileTable::new(),
            curr_dir: String::from("/"),
            heap: 0,
            children: Vec::new(),
            entry: 0,
            tms: Default::default(),
            rlimits: rlimits_new(),
            sigaction: [SigAction::new(); 64],
            futex_table: Arc::new(Mutex::new(BTreeMap::new())),
        };

        let tcb = RwLock::new(ThreadControlBlock {
            cx: Context::new(),
            exit_code: None,
            sigmask: SigProcMask::new(),
            clear_child_tid: 0,
            set_child_tid: 0,
            signal: SignalList::new(),
        });

        Arc::new(Self {
            page_table: PageTable::from_ppn(ppn.0),
            task_id,
            parent,
            pcb: Arc::new(Mutex::new(inner)),
            tcb,
        })
    }

    pub fn inner_map<T>(&self, mut f: impl FnMut(&mut MutexGuard<TaskInner>) -> T) -> T {
        f(&mut self.pcb.lock())
    }

    pub fn map(&self, ppn: PhysPage, vpn: VirtPage, flags: PTEFlags) {
        self.page_table.map(
            ppn,
            vpn,
            flags,
            || self.frame_alloc(VirtPage::new(0), MemType::PTE, 1),
            3,
        );
    }

    pub fn frame_alloc(&self, vpn: VirtPage, mtype: MemType, count: usize) -> PhysPage {
        self.map_frames(vpn, mtype, count, None, 0, 0)
    }

    pub fn map_frames(
        &self,
        vpn: VirtPage,
        mtype: MemType,
        count: usize,
        file: Option<File>,
        start: usize,
        len: usize,
    ) -> PhysPage {
        assert!(count > 0, "can't alloc count = 0 in user_task frame_alloc");
        // alloc trackers and map vpn
        let trackers: Vec<_> = frame_alloc_much(count)
            .expect("can't alloc frame in user_task")
            .into_iter()
            .enumerate()
            .map(|(i, x)| {
                let vpn = match vpn.to_addr() == 0 {
                    true => vpn,
                    false => vpn.add(i),
                };
                MapTrack {
                    vpn,
                    tracker: Arc::new(x),
                }
            })
            .collect();
        if vpn.to_addr() != 0 {
            debug!(
                "map {:#x} @ {:#x} size: {:#x} flags: {:?}",
                vpn.to_addr(),
                trackers[0].tracker.0.to_addr(),
                count * PAGE_SIZE,
                PTEFlags::UVRWX
            );
        }
        let mut inner = self.pcb.lock();

        // find map area, such as PTE, CodeSection, etc.
        let finded_area = inner
            .memset
            .iter_mut()
            .filter(|x| x.mtype != MemType::ShareFile || x.mtype != MemType::Shared)
            .find(|x| x.mtype == mtype);

        // add tracker and map memory.
        match finded_area {
            Some(area) => {
                for target in trackers.clone() {
                    area.map(target.vpn, target.tracker)
                }
            }
            None => inner.memset.push(MemArea {
                mtype,
                mtrackers: trackers.clone(),
                file,
                start,
                len,
            }),
        }
        drop(inner);

        // map vpn to ppn
        trackers
            .clone()
            .iter()
            .filter(|x| x.vpn.to_addr() != 0)
            .for_each(|x| self.map(x.tracker.0, x.vpn, PTEFlags::UVRWX));
        let ppn = trackers[0].tracker.0.clone();
        ppn
    }

    pub fn get_cx_ptr(&self) -> *mut Context {
        (&mut self.tcb.write().cx) as *mut Context
    }

    pub fn exit_code(&self) -> Option<usize> {
        self.tcb.read().exit_code
    }

    pub fn sbrk(&self, incre: isize) -> usize {
        let inner = self.pcb.lock();
        let curr_page = inner.heap / PAGE_SIZE;
        let after_page = (inner.heap as isize + incre) as usize / PAGE_SIZE;
        drop(inner);
        // need alloc frame page
        if after_page > curr_page {
            for i in curr_page..after_page {
                self.frame_alloc(VirtPage::new(i + 1), MemType::CodeSection, 1);
            }
        }
        let mut inner = self.pcb.lock();
        inner.heap = (inner.heap as isize + incre) as usize;
        inner.heap
    }

    pub fn heap(&self) -> usize {
        self.pcb.lock().heap
    }

    #[inline]
    pub fn exit(&self, exit_code: usize) {
        let mut tcb_writer = self.tcb.write();
        let uaddr = tcb_writer.clear_child_tid;
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
                futex_wake(self.pcb.lock().futex_table.clone(), uaddr, 1);
            }
        }
        tcb_writer.exit_code = Some(exit_code);
        drop(tcb_writer);
        FUTURE_LIST.lock().remove(&self.task_id);
        // recycle memory resouces if the pcb just used by this thread
        if Arc::strong_count(&self.pcb) == 1 {
            self.pcb.lock().memset.retain(|x| x.mtype != MemType::PTE);
            self.pcb.lock().fd_table.clear();
        }

        if let Some(parent) = self.parent.upgrade() {
            parent.as_user_task().map(|x| {
                x.tcb.write().signal.add_signal(SignalFlags::SIGCHLD);
            });
        } else {
            self.pcb.lock().children.clear();
        }
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
        let mut new_tcb_writer = new_task.tcb.write();
        // clone fd_table and clone heap
        let mut new_pcb = new_task.pcb.lock();
        let mut pcb = self.pcb.lock();

        new_pcb.fd_table.0 = pcb.fd_table.0.clone();
        new_pcb.heap = pcb.heap;

        new_tcb_writer.cx.clone_from(&self.tcb.read().cx);
        new_tcb_writer.cx.set_ret(0);
        new_pcb.curr_dir = pcb.curr_dir.clone();

        pcb.children.push(new_task.clone());
        drop(new_pcb);
        pcb.memset
            .iter()
            .filter(|x| x.mtype != MemType::PTE)
            .for_each(|x| {
                let map_area = x.fork();
                map_area.mtrackers.iter().for_each(|map_track| {
                    new_task.map(map_track.tracker.0, map_track.vpn, PTEFlags::UVRWX);
                });

                new_task.pcb.lock().memset.push(map_area);
            });
        drop(new_tcb_writer);
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
        let parent_tcb = self.tcb.read();
        let parent_task: Arc<dyn AsyncTask> = self.clone();

        let task_id = task_id_alloc();
        let mut pcb = self.pcb.lock();
        let tcb = RwLock::new(ThreadControlBlock {
            cx: parent_tcb.cx.clone(),
            exit_code: None,
            sigmask: parent_tcb.sigmask.clone(),
            clear_child_tid: 0,
            set_child_tid: 0,
            signal: SignalList::new(),
        });

        tcb.write().cx.set_ret(0);
        drop(parent_tcb);

        let new_task = Arc::new(Self {
            page_table: self.page_table.clone(),
            task_id,
            parent: Arc::downgrade(&parent_task),
            pcb: self.pcb.clone(),
            tcb,
        });
        pcb.children.push(new_task.clone());

        FUTURE_LIST.lock().insert(task_id, Pin::from(future));

        thread::spawn(new_task.clone());
        new_task
    }

    pub fn push_str(&self, str: &str) -> usize {
        let mut tcb = self.tcb.write();

        const ULEN: usize = size_of::<usize>();
        let bytes = str.as_bytes();
        let len = bytes.len();
        let sp = tcb.cx.sp() - (len + ULEN) / ULEN * ULEN;

        let phys_sp = paddr_c(self.page_table.virt_to_phys(VirtAddr::new(sp)));
        unsafe {
            core::slice::from_raw_parts_mut(phys_sp.addr() as *mut u8, len).copy_from_slice(bytes);
        }
        tcb.cx.set_sp(sp);
        sp
    }

    pub fn push_arr(&self, buffer: &[u8]) -> usize {
        let mut tcb = self.tcb.write();

        const ULEN: usize = size_of::<usize>();
        let len = buffer.len();
        let sp = tcb.cx.sp() - ceil_div(len, ULEN) * ULEN;

        let phys_sp = paddr_c(self.page_table.virt_to_phys(VirtAddr::new(sp)));
        unsafe {
            core::slice::from_raw_parts_mut(phys_sp.addr() as *mut u8, len).copy_from_slice(buffer);
        }
        tcb.cx.set_sp(sp);
        sp
    }

    pub fn push_num(&self, num: usize) -> usize {
        let mut tcb = self.tcb.write();

        const ULEN: usize = size_of::<usize>();
        let sp = tcb.cx.sp() - ULEN;

        let phys_sp = paddr_c(self.page_table.virt_to_phys(VirtAddr::new(sp)));

        unsafe {
            (phys_sp.addr() as *mut usize).write(num);
        }
        tcb.cx.set_sp(sp);
        sp
    }

    pub fn get_last_free_addr(&self) -> VirtAddr {
        VirtAddr::new(
            self.pcb
                .lock()
                .memset
                .iter()
                .filter(|x| x.mtype != MemType::Stack)
                .fold(0, |acc, x| {
                    x.mtrackers
                        .iter()
                        .filter(|x| x.vpn.to_addr() > acc)
                        .map(|x| x.vpn.to_addr())
                        .max()
                        .unwrap_or(acc)
                })
                + PAGE_SIZE,
        )
    }

    pub fn get_fd(&self, index: usize) -> Option<File> {
        let inner = self.pcb.lock();
        match index >= inner.rlimits[7] {
            true => None,
            false => inner.fd_table.0[index].clone(),
        }
    }

    pub fn set_fd(&self, index: usize, value: Option<File>) {
        let mut inner = self.pcb.lock();
        match index >= inner.rlimits[7] {
            true => {}
            false => inner.fd_table.0[index] = value,
        }
    }

    pub fn alloc_fd(&self) -> Option<usize> {
        let mut inner = self.pcb.lock();
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
        self.pcb
            .lock()
            .fd_table
            .0
            .iter()
            .filter(|x| x.is_some())
            .count()
    }
}

impl AsyncTask for UserTask {
    #[inline]
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
