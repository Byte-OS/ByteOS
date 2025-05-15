use alloc::{
    collections::VecDeque,
    sync::{Arc, Weak},
    task::Wake,
};
use core::task::{Context, Poll};
use hashbrown::HashMap;
use sync::{LazyInit, Mutex};

use crate::task::{AsyncTask, AsyncTaskItem, PinedFuture};

pub type TaskId = usize;

pub static TASK_MAP: LazyInit<Mutex<HashMap<usize, Weak<dyn AsyncTask>>>> = LazyInit::new();
/// FIFO task queue, Items will be pushed to the end of the queue after being called.
pub(crate) static TASK_QUEUE: Mutex<VecDeque<AsyncTaskItem>> = Mutex::new(VecDeque::new());
/// wake queue, not use at current.
pub(crate) static CURRENT_TASK: LazyInit<Mutex<Arc<dyn AsyncTask>>> = LazyInit::new();

pub static DEFAULT_EXECUTOR: Executor = Executor::new();

pub struct Executor {}

impl Executor {
    pub const fn new() -> Self {
        Executor {}
    }

    pub fn init(&self) {
        // Init TaskMAP with new empty hash map
        TASK_MAP.init_by(Mutex::new(HashMap::new()));
    }

    pub fn spawn(&mut self, task: Arc<dyn AsyncTask>, future: PinedFuture) {
        TASK_QUEUE.lock().push_back(AsyncTaskItem { future, task })
    }

    pub fn run(&self) {
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
    CURRENT_TASK.lock().clone()
}
