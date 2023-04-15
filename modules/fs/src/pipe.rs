use core::cmp;

use alloc::{collections::VecDeque, sync::Arc};
use sync::Mutex;
use vfscore::INodeInterface;

// pipe sender, just can write.
pub struct PipeSender(Arc<Mutex<VecDeque<u8>>>);

impl INodeInterface for PipeSender {
    fn write(&self, buffer: &[u8]) -> vfscore::VfsResult<usize> {
        let mut queue = self.0.lock();
        let wlen = buffer.len();
        queue.extend(buffer.iter());
        Ok(wlen)
    }
}

// pipe reader, just can read.
pub struct PipeReceiver(Arc<Mutex<VecDeque<u8>>>);

impl INodeInterface for PipeReceiver {
    fn read(&self, buffer: &mut [u8]) -> vfscore::VfsResult<usize> {
        let mut queue = self.0.lock();
        let rlen = cmp::min(queue.len(), buffer.len());
        queue
            .drain(..rlen)
            .enumerate()
            .into_iter()
            .for_each(|(i, x)| {
                buffer[i] = x;
            });
        Ok(rlen)
    }
}

pub fn create_pipe() -> (PipeReceiver, PipeSender) {
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    (PipeReceiver(queue.clone()), PipeSender(queue.clone()))
}
