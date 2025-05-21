use core::{future::Future, pin::Pin};

use alloc::{boxed::Box, sync::Arc};
use downcast_rs::DowncastSync;

use crate::TaskId;

/// Default is kernel task
pub const TYPE_KERNEL_TASK: u8 = 0;

/// This is a trait the for generic task.
pub trait AsyncTask: DowncastSync {
    /// Get the id of the task
    fn get_task_id(&self) -> TaskId;
    /// Run befire the kernel
    fn before_run(&self);
    /// Get task type.
    /// Exit a task with exit code.
    fn exit(&self, exit_code: usize);
    /// Check if the task was exited successfully
    fn exit_code(&self) -> Option<usize>;
}

pub type PinedFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// This is a async task container will be called in the Async Task Item
pub struct AsyncTaskItem {
    pub future: PinedFuture,
    pub task: Arc<dyn AsyncTask>,
}
