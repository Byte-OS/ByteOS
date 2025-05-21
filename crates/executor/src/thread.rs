use core::future::Future;

use alloc::{boxed::Box, sync::Arc};

use crate::{
    task::{AsyncTask, AsyncTaskItem},
    TASK_MAP, TASK_QUEUE,
};

#[inline]
pub fn spawn(task: Arc<dyn AsyncTask>, future: impl Future<Output = ()> + Send + 'static) {
    TASK_MAP
        .lock()
        .insert(task.get_task_id(), Arc::downgrade(&task));
    TASK_QUEUE.lock().push_back(AsyncTaskItem {
        future: Box::pin(future),
        task,
    });
}
