use alloc::{
    collections::VecDeque,
    sync::{Arc, Weak},
    task::Wake,
    vec::Vec,
};
use core::{
    hint::spin_loop,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};
use hashbrown::HashMap;
use log::info;
use polyhal::{hart_id, PageTable};
use sync::{LazyInit, Mutex};

use crate::task::{AsyncTask, AsyncTaskItem, PinedFuture};

pub type TaskId = usize;

pub static TASK_MAP: LazyInit<Mutex<HashMap<usize, Weak<dyn AsyncTask>>>> = LazyInit::new();
/// FIFO task queue, Items will be pushed to the end of the queue after being called.
pub(crate) static TASK_QUEUE: Mutex<VecDeque<AsyncTaskItem>> = Mutex::new(VecDeque::new());

pub static DEFAULT_EXECUTOR: Executor = Executor::new();

static BOOT_PAGE: LazyInit<PageTable> = LazyInit::new();

pub struct Executor {
    cores: LazyInit<Vec<Mutex<Option<Arc<dyn AsyncTask>>>>>,
    inited: AtomicBool,
}

impl Executor {
    pub const fn new() -> Self {
        Executor {
            cores: LazyInit::new(),
            inited: AtomicBool::new(false),
        }
    }

    pub fn init(&self, cores: usize) {
        let mut core_container = Vec::with_capacity(cores);
        (0..cores).for_each(|_| core_container.push(Mutex::new(None)));
        self.cores.init_by(core_container);

        // Init TaskMAP with new empty hash map
        TASK_MAP.init_by(Mutex::new(HashMap::new()));
        if !BOOT_PAGE.is_init() {
            BOOT_PAGE.init_by(PageTable::current());
        }

        // Finish initializing
        self.inited.store(true, Ordering::SeqCst);
    }

    pub fn spawn(&mut self, task: Arc<dyn AsyncTask>, future: PinedFuture) {
        TASK_QUEUE.lock().push_back(AsyncTaskItem { future, task })
    }

    pub fn run(&self) {
        info!("fetch atomic data: {}", self.inited.load(Ordering::SeqCst));
        info!(
            "fetch atomic data not: {}",
            self.inited.load(Ordering::SeqCst)
        );
        // Waiting for executor's initialisation finish.
        while !self.inited.load(Ordering::SeqCst) {
            spin_loop();
        }
        loop {
            self.run_ready_task();
            self.hlt_if_idle();
        }
    }

    fn run_ready_task(&self) {
        let task = TASK_QUEUE.lock().pop_front();
        if let Some(task_item) = task {
            let AsyncTaskItem { task, mut future } = task_item;
            task.before_run();
            *self.cores[hart_id()].lock() = Some(task.clone());
            // Create Waker
            let waker = Arc::new(Waker {
                task_id: task.get_task_id(),
            })
            .into();
            let mut context = Context::from_waker(&waker);

            match future.as_mut().poll(&mut context) {
                Poll::Ready(()) => {} // task done
                Poll::Pending => TASK_QUEUE.lock().push_back(AsyncTaskItem { future, task }),
            }
        }
    }

    /// Executes the `hlt` instruction if there are no ready tasks
    fn hlt_if_idle(&self) {
        // arch::wfi();
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
pub struct Waker {
    task_id: TaskId,
}

impl Wake for Waker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {}
}

/// Alloc a task id.
pub fn task_id_alloc() -> TaskId {
    static TASK_ID: Mutex<usize> = Mutex::new(0);
    let mut task_id = TASK_ID.lock();
    *task_id += 1;
    *task_id
}

/// Get task through task id.
pub fn tid2task(tid: usize) -> Option<Arc<dyn AsyncTask>> {
    TASK_MAP.lock().get(&tid).cloned().map(|x| x.upgrade())?
}

/// Release task
pub fn release_task(tid: usize) {
    // Remove task from TASK_MAP
    TASK_MAP.lock().remove(&tid);
}

#[inline]
pub fn current_task() -> Arc<dyn AsyncTask> {
    // CURRENT_TASK.lock().as_ref().map(|x| x.clone()).unwrap()
    DEFAULT_EXECUTOR.cores[hart_id()]
        .lock()
        .as_ref()
        .map(|x| x.clone())
        .unwrap()
}

pub fn boot_page_table() -> PageTable {
    *BOOT_PAGE
}
