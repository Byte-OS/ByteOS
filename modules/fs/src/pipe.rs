use core::cmp;

use alloc::{
    collections::VecDeque,
    sync::{Arc, Weak},
};
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

    fn fcntl(&self, _cmd: usize, _arg: usize) -> vfscore::VfsResult<()> {
        info!("pipe sender fcntl, cmd: {}, _arg: {}", _cmd, _arg);
        Err(vfscore::VfsError::NotSupported)
    }
}

// pipe reader, just can read.
pub struct PipeReceiver {
    queue: Arc<Mutex<VecDeque<u8>>>,
    sender: Weak<PipeSender>,
}

impl INodeInterface for PipeReceiver {
    fn read(&self, buffer: &mut [u8]) -> vfscore::VfsResult<usize> {
        let mut queue = self.queue.lock();
        let rlen = cmp::min(queue.len(), buffer.len());
        queue
            .drain(..rlen)
            .enumerate()
            .into_iter()
            .for_each(|(i, x)| {
                buffer[i] = x;
            });
        if rlen == 0 && Weak::strong_count(&self.sender) > 0 {
            Err(vfscore::VfsError::Blocking)
        } else {
            Ok(rlen)
        }
    }

    fn fcntl(&self, _cmd: usize, _arg: usize) -> vfscore::VfsResult<()> {
        info!("pipe receiver fcntl, cmd: {}, _arg: {}", _cmd, _arg);
        Err(vfscore::VfsError::NotSupported)
    }
}

pub fn create_pipe() -> (Arc<PipeReceiver>, Arc<PipeSender>) {
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    let sender = Arc::new(PipeSender(queue.clone()));
    (
        Arc::new(PipeReceiver {
            queue: queue.clone(),
            sender: Arc::downgrade(&sender),
        }),
        sender,
    )
}
