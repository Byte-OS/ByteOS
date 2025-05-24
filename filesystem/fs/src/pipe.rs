use core::cmp;

use alloc::{
    collections::VecDeque,
    sync::{Arc, Weak},
};
use libc_types::{
    poll::PollEvent,
    types::{Stat, StatMode},
};
use sync::Mutex;
use syscalls::Errno;
use vfscore::{INodeInterface, VfsResult};

pub struct PipeSender(Arc<Mutex<VecDeque<u8>>>);

impl INodeInterface for PipeSender {
    fn writeat(&self, _offset: usize, buffer: &[u8]) -> VfsResult<usize> {
        log::warn!("write pipe:");
        let mut queue = self.0.lock();
        if queue.len() > 0x50000 {
            Err(Errno::EWOULDBLOCK)
        } else {
            let wlen = buffer.len();
            queue.extend(buffer.iter());
            Ok(wlen)
        }
    }

    fn poll(&self, events: PollEvent) -> VfsResult<PollEvent> {
        let mut res = PollEvent::NONE;
        if events.contains(PollEvent::OUT) && self.0.lock().len() <= 0x50000 {
            res |= PollEvent::OUT;
        }
        Ok(res)
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.mode = StatMode::FIFO;
        Ok(())
    }
}

// pipe reader, just can read.
pub struct PipeReceiver {
    queue: Arc<Mutex<VecDeque<u8>>>,
    sender: Weak<PipeSender>,
}

impl INodeInterface for PipeReceiver {
    fn readat(&self, _offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        let mut queue = self.queue.lock();
        let rlen = cmp::min(queue.len(), buffer.len());
        queue
            .drain(..rlen)
            .zip(buffer.iter_mut())
            .for_each(|(src, dst)| *dst = src);
        if rlen == 0 && Weak::strong_count(&self.sender) > 0 {
            Err(Errno::EWOULDBLOCK)
        } else {
            Ok(rlen)
        }
    }

    fn poll(&self, events: PollEvent) -> VfsResult<PollEvent> {
        let mut res = PollEvent::NONE;
        if events.contains(PollEvent::IN) {
            if self.queue.lock().len() > 0 {
                res |= PollEvent::IN;
            } else if Weak::strong_count(&self.sender) == 0 {
                res |= PollEvent::ERR;
            }
        }
        if events.contains(PollEvent::ERR)
            && self.queue.lock().len() == 0
            && Weak::strong_count(&self.sender) == 0
        {
            res |= PollEvent::ERR;
        }
        Ok(res)
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.mode = StatMode::FIFO;
        Ok(())
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
