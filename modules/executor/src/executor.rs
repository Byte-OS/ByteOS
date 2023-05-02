use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    task::Wake,
};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::SegQueue;
use sync::Mutex;

use crate::UserTask;

pub trait AsyncTask: Send + Sync {
    fn get_task_id(&self) -> TaskId;
    fn before_run(&self);
    fn as_user_task(self: Arc<Self>) -> Option<Arc<UserTask>> {
        None
    }
}

pub type TaskId = usize;
type PinedFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
pub static CURRENT_TASK: Mutex<Option<Arc<dyn AsyncTask>>> = Mutex::new(None);

pub static FUTURE_LIST: Mutex<BTreeMap<usize, PinedFuture>> = Mutex::new(BTreeMap::new());
pub static TASK_QUEUE: Mutex<VecDeque<Arc<dyn AsyncTask>>> = Mutex::new(VecDeque::new());
/// wake queue, not use at current.
pub static WAKE_QUEUE: SegQueue<TaskId> = SegQueue::new();
pub struct Executor;

impl Executor {
    pub fn new() -> Self {
        Executor
    }

    pub fn spawn(&mut self, task: Arc<dyn AsyncTask>) {
        TASK_QUEUE.lock().push_back(task)
    }

    pub fn run(&mut self) {
        loop {
            if TASK_QUEUE.lock().len() == 0 {
                break;
            }
            self.run_ready_task();
            self.hlt_if_idle();
        }
    }

    fn run_ready_task(&mut self) {
        let task = TASK_QUEUE.lock().pop_front();
        if let Some(task) = task {
            task.before_run();
            *CURRENT_TASK.lock() = Some(task.clone());
            let waker = self.create_waker(task.as_ref()).into();
            let mut context = Context::from_waker(&waker);

            let future = FUTURE_LIST.lock().remove(&task.get_task_id());

            if let Some(mut future) = future {
                match future.as_mut().poll(&mut context) {
                    Poll::Ready(()) => {} // task done
                    Poll::Pending => TASK_QUEUE.lock().push_back(task.clone()),
                }
                FUTURE_LIST.lock().insert(task.get_task_id(), future);
            }
        }
    }

    /// Executes the `hlt` instruction if there are no ready tasks
    fn hlt_if_idle(&self) {
        if TASK_QUEUE.lock().len() == 0 {
            arch::wfi();
        }
    }

    fn task_id(task: &dyn AsyncTask) -> TaskId {
        task.get_task_id()
    }

    fn create_waker(&self, task: &dyn AsyncTask) -> Arc<Waker> {
        Arc::new(Waker {
            task_id: Self::task_id(task),
        })
    }
}

pub struct Waker {
    task_id: TaskId,
}

impl Wake for Waker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        WAKE_QUEUE.push(self.task_id);
    }
}

/// Alloc a task id.
pub fn task_id_alloc() -> TaskId {
    static TASK_ID: Mutex<usize> = Mutex::new(0);
    let mut task_id = TASK_ID.lock();
    *task_id += 1;
    *task_id
}

pub fn current_task() -> Arc<dyn AsyncTask> {
    CURRENT_TASK.lock().as_ref().map(|x| x.clone()).unwrap()
}

pub fn current_user_task() -> Arc<UserTask> {
    CURRENT_TASK
        .lock()
        .as_ref()
        .map(|x| x.clone().as_user_task().unwrap())
        .unwrap()
}
