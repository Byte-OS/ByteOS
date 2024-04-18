use alloc::sync::Arc;

use crate::{AsyncTask, TASK_QUEUE};

#[inline]
pub fn spawn(task: Arc<dyn AsyncTask>) {
    TASK_QUEUE.lock().push_back(task);
}
