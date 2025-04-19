use core::{future::Future, pin::Pin};

use alloc::{boxed::Box, sync::Arc};
use downcast_rs::{impl_downcast, DowncastSync};

use crate::{boot_page_table, TaskId};

/// Default is kernel task
pub const TYPE_KERNEL_TASK: u8 = 0;

/// This is a trait the for generic task.
pub trait AsyncTask: DowncastSync {
    /// Get the id of the task
    fn get_task_id(&self) -> TaskId;
    /// Run befire the kernel
    fn before_run(&self);
    /// Get task type.
    fn get_task_type(&self) -> TaskType;
    /// Exit a task with exit code.
    fn exit(&self, exit_code: usize);
    /// Check if the task was exited successfully
    fn exit_code(&self) -> Option<usize>;
}

/// This is a enum that indicates the task type.
#[derive(Debug, PartialEq, PartialOrd)]
pub enum TaskType {
    /// Blank Kernel Task Type, Just run in the kernel,
    /// No extra pagetable
    BlankKernel,
    /// Monolithic Task Type, Will have a independent pagetable.
    MonolithicTask,
    /// Microkernel task
    MicroTask,
    /// Unikernel task
    UnikernelTask,
    /// RTOS task
    RTOSTask,
    /// User defined task 1
    UserDefinedTask1,
    /// User defined task 2
    UserDefinedTask2,
}

pub type PinedFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// This is a async task container will be called in the Async Task Item
pub struct AsyncTaskItem {
    pub future: PinedFuture,
    pub task: Arc<dyn AsyncTask>,
}

/// This is a blank kernel task.
pub struct BlankKernelTask(pub usize);
impl AsyncTask for BlankKernelTask {
    /// Get task identifier
    fn get_task_id(&self) -> TaskId {
        self.0
    }

    /// before run switch to kernel page table.
    /// maybe I don't need to do this.
    fn before_run(&self) {
        boot_page_table().change();
    }

    /// Get task type.
    fn get_task_type(&self) -> TaskType {
        TaskType::BlankKernel
    }

    /// Exit a task with exit code. But kernel blanktask's exit function never be called.
    fn exit(&self, _exit_code: usize) {
        unreachable!("can't exit blank kernel task")
    }

    /// Get the task exit code.
    fn exit_code(&self) -> Option<usize> {
        unreachable!("Kernel blanktask can't exit")
    }
}

impl_downcast!(sync AsyncTask);
