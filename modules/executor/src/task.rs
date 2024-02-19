use core::{future::Future, mem::size_of, ops::Add};

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    sync::{Arc, Weak},
    vec::Vec,
};
use arch::{
    Context, ContextOps, PTEFlags, PageTable, PhysPage, VirtAddr, VirtPage, PAGE_SIZE,
};
use frame_allocator::{ceil_div, frame_alloc, frame_alloc_much, FrameTracker};
use fs::File;
use log::{debug, warn};
use signal::REAL_TIME_SIGNAL_NUM;
pub use signal::{SigAction, SigProcMask, SignalFlags};
use sync::{Mutex, MutexGuard, RwLock};
use vfscore::OpenFlags;

use crate::{
    filetable::{rlimits_new, FileItem, FileTable},
    memset::{MapTrack, MemArea, MemType},
    shm::MapedSharedMemory,
    signal::SignalList,
    task_id_alloc, thread, AsyncTask, MemSet, ProcessTimer, TaskFutureItem, TaskId, FUTURE_LIST,
    TMS,
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
    pub fn new(future: impl Future<Output = ()> + 'static) -> Arc<Self> {
        let ppn = Arc::new(frame_alloc().unwrap());
        let page_table = PageTable::from_ppn(ppn.0);
        let task_id = task_id_alloc();
        let memset = vec![ppn];

        FUTURE_LIST
            .lock()
            .insert(task_id, TaskFutureItem(Box::pin(kernel_entry(future))));

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

pub struct ProcessControlBlock {
    pub memset: MemSet,
    pub fd_table: FileTable,
    pub curr_dir: Arc<FileItem>,
    pub heap: usize,
    pub entry: usize,
    pub children: Vec<Arc<UserTask>>,
    pub tms: TMS,
    pub rlimits: Vec<usize>,
    pub sigaction: [SigAction; 65],
    pub futex_table: Arc<Mutex<FutexTable>>,
    pub shms: Vec<MapedSharedMemory>,
    pub timer: [ProcessTimer; 3],
    pub threads: Vec<Weak<UserTask>>,
    pub exit_code: Option<usize>,
}

pub struct ThreadControlBlock {
    pub cx: Context,
    pub sigmask: SigProcMask,
    pub clear_child_tid: usize,
    pub set_child_tid: usize,
    pub signal: SignalList,
    pub signal_queue: [usize; REAL_TIME_SIGNAL_NUM], // a queue for real time signals
    pub exit_signal: u8,
    pub thread_exit_code: Option<u32>,
}

#[allow(dead_code)]
pub struct UserTask {
    pub task_id: TaskId,
    pub process_id: TaskId,
    pub page_table: PageTable,
    pub pcb: Arc<Mutex<ProcessControlBlock>>,
    pub parent: RwLock<Weak<dyn AsyncTask>>,
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
        future: impl Future<Output = ()> + 'static,
        parent: Weak<dyn AsyncTask>,
    ) -> Arc<Self> {
        let ppn = Arc::new(frame_alloc().unwrap());
        let task_id = task_id_alloc();
        // initialize memset
        let memset = MemSet::new(vec![MemArea::new(
            MemType::PTE,
            vec![MapTrack {
                vpn: VirtPage::new(0),
                tracker: ppn.clone(),
                rwx: 0,
            }],
            0,
            0,
        )]);

        FUTURE_LIST
            .lock()
            .insert(task_id, TaskFutureItem(Box::pin(future)));

        let inner = ProcessControlBlock {
            memset,
            fd_table: FileTable::new(),
            curr_dir: FileItem::fs_open("/tmp_home", OpenFlags::all())
                .expect("dont' have the home dir"),
            heap: 0,
            children: Vec::new(),
            entry: 0,
            tms: Default::default(),
            rlimits: rlimits_new(),
            sigaction: [SigAction::new(); 65],
            futex_table: Arc::new(Mutex::new(BTreeMap::new())),
            shms: vec![],
            timer: [Default::default(); 3],
            exit_code: None,
            threads: Vec::new(),
        };

        let tcb = RwLock::new(ThreadControlBlock {
            cx: Context::new(),
            sigmask: SigProcMask::new(),
            clear_child_tid: 0,
            set_child_tid: 0,
            signal: SignalList::new(),
            signal_queue: [0; REAL_TIME_SIGNAL_NUM],
            exit_signal: 0,
            thread_exit_code: Option::None,
        });

        let task = Arc::new(Self {
            page_table: PageTable::from_ppn(ppn.0),
            task_id,
            process_id: task_id,
            parent: RwLock::new(parent),
            pcb: Arc::new(Mutex::new(inner)),
            tcb,
        });
        task.pcb.lock().threads.push(Arc::downgrade(&task));
        task
    }

    pub fn inner_map<T>(&self, mut f: impl FnMut(&mut MutexGuard<ProcessControlBlock>) -> T) -> T {
        f(&mut self.pcb.lock())
    }

    pub fn map(&self, ppn: PhysPage, vpn: VirtPage, flags: PTEFlags) {
        self.page_table.map(
            ppn,
            vpn,
            flags,
            || {
                self.frame_alloc(VirtPage::new(0), MemType::PTE, 1)
                    .expect("can't alloc page in map")
            },
            3,
        );
    }

    pub fn frame_alloc(&self, vpn: VirtPage, mtype: MemType, count: usize) -> Option<PhysPage> {
        self.map_frames(vpn, mtype, count, None, 0, vpn.to_addr(), count * PAGE_SIZE)
    }

    pub fn map_frames(
        &self,
        vpn: VirtPage,
        mtype: MemType,
        count: usize,
        file: Option<File>,
        offset: usize,
        start: usize,
        len: usize,
    ) -> Option<PhysPage> {
        assert!(count > 0, "can't alloc count = 0 in user_task frame_alloc");
        // alloc trackers and map vpn
        let trackers: Vec<_> = frame_alloc_much(count)?
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
                    rwx: 0,
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
            // map vpn to ppn
            trackers
                .clone()
                .iter()
                .filter(|x| x.vpn.to_addr() != 0)
                .for_each(|x| self.map(x.tracker.0, x.vpn, PTEFlags::UVRWX));
        }
        let mut inner = self.pcb.lock();
        let ppn = trackers[0].tracker.0;
        if mtype == MemType::PTE || mtype == MemType::Stack {
            let finded_area = inner.memset.iter_mut().find(|x| x.mtype == mtype);
            if let Some(area) = finded_area {
                area.mtrackers.extend(trackers);
            } else if mtype == MemType::PTE {
                inner.memset.push(MemArea::new(mtype, trackers, start, len));
            } else if mtype == MemType::Stack {
                inner.memset.push(MemArea {
                    mtype,
                    mtrackers: trackers.clone(),
                    file: None,
                    offset: 0,
                    start: 0x7000_0000,
                    len: 0x1000_0000,
                });
            }
        } else {
            inner.memset.push(MemArea {
                mtype,
                mtrackers: trackers.clone(),
                file,
                offset,
                start,
                len,
            });
        }
        drop(inner);

        Some(ppn)
    }

    // pub fn get_cx_ptr(&self) -> *mut Context {
    //     // (&mut self.tcb.read().cx) as *mut Context
    //     unsafe { &mut self.tcb.as_mut_ptr().as_mut().unwrap().cx as _ }
    // }

    pub fn force_cx_ref(&self) -> &'static mut Context {
        unsafe { &mut self.tcb.as_mut_ptr().as_mut().unwrap().cx }
    }

    pub fn exit_code(&self) -> Option<usize> {
        self.pcb.lock().exit_code
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
        let tcb_writer = self.tcb.write();
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
                addr.get_mut_ptr::<u32>().write(0);
                futex_wake(self.pcb.lock().futex_table.clone(), uaddr, 1);
            }
        }
        self.pcb.lock().exit_code = Some(exit_code);
        let exit_signal = tcb_writer.exit_signal;
        drop(tcb_writer);
        FUTURE_LIST.lock().remove(&self.task_id);

        // recycle memory resouces if the pcb just used by this thread
        if Arc::strong_count(&self.pcb) == 1 {
            self.pcb.lock().memset.retain(|x| x.mtype == MemType::PTE);
            self.pcb.lock().fd_table.clear();
            self.pcb.lock().children.clear();
        }

        if let Some(parent) = self.parent.read().upgrade() {
            parent.as_user_task().map(|x| {
                if exit_signal != 0 {
                    x.tcb
                        .write()
                        .signal
                        .add_signal(SignalFlags::from_usize(exit_signal as usize));
                } else {
                    x.tcb.write().signal.add_signal(SignalFlags::SIGCHLD);
                }
            });
        } else {
            self.pcb.lock().children.clear();
        }
    }

    #[inline]
    pub fn thread_exit(&self, exit_code: usize) {
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
                addr.get_mut_ptr::<u32>().write(0);
                futex_wake(self.pcb.lock().futex_table.clone(), uaddr, 1);
            }
        }
        tcb_writer.thread_exit_code = Some(exit_code as u32);
        let exit_signal = tcb_writer.exit_signal;
        drop(tcb_writer);
        FUTURE_LIST.lock().remove(&self.task_id);

        // recycle memory resouces if the pcb just used by this thread
        if Arc::strong_count(&self.pcb) == 1 {
            self.pcb.lock().memset.retain(|x| x.mtype == MemType::PTE);
            self.pcb.lock().fd_table.clear();
            self.pcb.lock().children.clear();
            self.pcb.lock().exit_code = Some(exit_code);
        }

        if let Some(parent) = self.parent.read().upgrade() {
            parent.as_user_task().map(|x| {
                if exit_signal != 0 {
                    x.tcb
                        .write()
                        .signal
                        .add_signal(SignalFlags::from_usize(exit_signal as usize));
                } else {
                    x.tcb.write().signal.add_signal(SignalFlags::SIGCHLD);
                }
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
    pub fn fork(self: Arc<Self>, future: impl Future<Output = ()> + 'static) -> Arc<Self> {
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
        new_pcb.shms = pcb.shms.clone();
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
        // map shms
        warn!("map shms");
        pcb.shms.iter().enumerate().for_each(|(i, x)| {
            new_task.map(
                x.mem.trackers[i].0,
                VirtPage::from_addr(x.start).add(i),
                PTEFlags::UVRWX,
            );
        });
        thread::spawn(new_task.clone());
        new_task
    }

    #[inline]
    pub fn cow_fork(self: Arc<Self>, future: impl Future<Output = ()> + 'static) -> Arc<Self> {
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
        new_pcb.shms = pcb.shms.clone();
        drop(new_pcb);
        // cow fork
        pcb.memset
            .iter()
            .filter(|x| x.mtype != MemType::PTE)
            .for_each(|x| {
                let map_area = x.clone();
                map_area.mtrackers.iter().for_each(|x| {
                    new_task.map(x.tracker.0, x.vpn, PTEFlags::UVRX);
                    self.map(x.tracker.0, x.vpn, PTEFlags::UVRX);
                });
                new_task.pcb.lock().memset.push(map_area);
            });
        drop(new_tcb_writer);
        // copy shm and map them
        pcb.shms.iter().for_each(|x| {
            x.mem.trackers.iter().enumerate().for_each(|(i, tracker)| {
                new_task.map(
                    tracker.0,
                    VirtPage::from_addr(x.start).add(i),
                    PTEFlags::UVRWX,
                );
            });
        });
        thread::spawn(new_task.clone());
        new_task
    }

    #[inline]
    pub fn thread_clone(self: Arc<Self>, future: impl Future<Output = ()> + 'static) -> Arc<Self> {
        // Give the frame_tracker in the memset a type.
        // it will contains the frame used for page mapping、
        // mmap or text section.
        // and then we can implement COW(copy on write).
        let parent_tcb = self.tcb.read();

        let task_id = task_id_alloc();
        let mut pcb = self.pcb.lock();
        let tcb = RwLock::new(ThreadControlBlock {
            cx: parent_tcb.cx.clone(),
            sigmask: parent_tcb.sigmask.clone(),
            clear_child_tid: 0,
            set_child_tid: 0,
            signal: SignalList::new(),
            signal_queue: [0; REAL_TIME_SIGNAL_NUM],
            exit_signal: 0,
            thread_exit_code: Option::None,
        });

        tcb.write().cx.set_ret(0);
        drop(parent_tcb);

        let new_task = Arc::new(Self {
            page_table: self.page_table.clone(),
            task_id,
            process_id: self.task_id,
            parent: RwLock::new(self.parent.read().clone()),
            pcb: self.pcb.clone(),
            tcb,
        });
        pcb.threads.push(Arc::downgrade(&new_task));
        // pcb.children.push(new_task.clone());

        FUTURE_LIST
            .lock()
            .insert(task_id, TaskFutureItem(Box::pin(future)));

        thread::spawn(new_task.clone());
        new_task
    }

    pub fn push_str(&self, str: &str) -> usize {
        self.push_arr(str.as_bytes())
    }

    pub fn push_arr(&self, buffer: &[u8]) -> usize {
        let mut tcb = self.tcb.write();

        const ULEN: usize = size_of::<usize>();
        let len = buffer.len();
        let sp = tcb.cx.sp() - ceil_div(len, ULEN) * ULEN;

        VirtAddr::from(sp).slice_mut_with_len(len).copy_from_slice(buffer);
        tcb.cx.set_sp(sp);
        sp
    }

    pub fn push_num(&self, num: usize) -> usize {
        let mut tcb = self.tcb.write();

        const ULEN: usize = size_of::<usize>();
        let sp = tcb.cx.sp() - ULEN;

        *VirtAddr::from(sp).get_mut_ref() = num;
        tcb.cx.set_sp(sp);
        sp
    }

    pub fn get_last_free_addr(&self) -> VirtAddr {
        // let map_last = self
        //     .pcb
        //     .lock()
        //     .memset
        //     .iter()
        //     .filter(|x| x.mtype != MemType::Stack)
        //     .fold(0, |acc, x| {
        //         x.mtrackers
        //             .iter()
        //             .filter(|x| x.vpn.to_addr() > acc && x.vpn.to_addr() < VIRT_ADDR_START)
        //             .map(|x| x.vpn.to_addr())
        //             .max()
        //             .unwrap_or(acc)
        //     })
        //     + PAGE_SIZE;
        let map_last = self
            .pcb
            .lock()
            .memset
            .iter()
            .filter(|x| x.mtype != MemType::Stack)
            .fold(0, |acc, x| {
                if acc > x.start + x.len {
                    acc
                } else {
                    x.start + x.len
                }
            });
        let shm_last = self.pcb.lock().shms.iter().fold(0, |acc, v| {
            if v.start + v.size > acc {
                v.start + v.size
            } else {
                acc
            }
        });

        VirtAddr::new(if map_last > shm_last {
            map_last
        } else {
            shm_last
        })
    }

    pub fn get_fd(&self, index: usize) -> Option<Arc<FileItem>> {
        let pcb = self.pcb.lock();
        match index >= pcb.rlimits[7] {
            true => None,
            false => pcb.fd_table.0[index].clone(),
        }
    }

    pub fn set_fd(&self, index: usize, value: Arc<FileItem>) {
        let mut pcb = self.pcb.lock();
        match index >= pcb.rlimits[7] {
            true => {}
            false => pcb.fd_table.0[index] = Some(value),
        }
    }

    pub fn clear_fd(&self, index: usize) {
        let mut pcb = self.pcb.lock();
        match index >= pcb.fd_table.len() {
            true => {}
            false => pcb.fd_table.0[index] = None,
        }
    }

    pub fn alloc_fd(&self) -> Option<usize> {
        let mut pcb = self.pcb.lock();
        let index = pcb
            .fd_table
            .0
            .iter()
            .enumerate()
            .find(|(i, x)| x.is_none() && *i < pcb.rlimits[7])
            .map(|(i, _)| i);
        if index.is_none() && pcb.fd_table.0.len() < pcb.rlimits[7] {
            pcb.fd_table.0.push(None);
            Some(pcb.fd_table.0.len() - 1)
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

pub async fn kernel_entry(future: impl Future<Output = ()> + 'static) {
    debug!("kernel_entry");
    future.await;
}
