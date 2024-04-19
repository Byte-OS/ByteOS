use alloc::sync::Arc;
use downcast_rs::{impl_downcast, DowncastSync};

use crate::TaskId;

/// Default is kernel task
pub const TYPE_KERNEL_TASK: u8 = 0;

// TODO: Use AsyncTask instead of AsyncTask Trait.
#[allow(dead_code)]
pub struct AsyncTask {
    /// Task id
    task_id: TaskId,
    /// Check the task type, help us to handle the situation that
    /// kernel task and monolithic task coexistence
    task_type: u8,
    /// Task extended data
    pub extend: Arc<dyn TaskExtend>,
}

/// Blank Task Extend.
/// Usually used in the kernel task.
/// But if you want to implement the unikernel
/// You should use another Task Extend
pub struct BlankTaskExtend;

/// implement task extend to blank
impl TaskExtend for BlankTaskExtend {}

pub trait TaskExtend: DowncastSync {
    /// blank before run
    fn before_run(&self) {}
}

impl_downcast!(sync TaskExtend);
