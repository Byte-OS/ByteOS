use core::cmp;

use alloc::{
    collections::VecDeque,
    sync::Arc,
};
use sync::Mutex;
use vfscore::{INodeInterface, PollEvent, VfsResult};

pub struct SocketPair {
    inner: Arc<Mutex<VecDeque<u8>>>
}

impl INodeInterface for SocketPair {
    fn writeat(&self, _offset: usize, buffer: &[u8]) -> VfsResult<usize> {
        let mut queue = self.inner.lock();
        if queue.len() > 0x50000 {
            Err(vfscore::VfsError::Blocking)
        } else {
            let wlen = buffer.len();
            queue.extend(buffer.iter());
            Ok(wlen)
        }
    }

    fn readat(&self, _offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        let mut queue = self.inner.lock();
        let rlen = cmp::min(queue.len(), buffer.len());
        queue
            .drain(..rlen)
            .enumerate()
            .into_iter()
            .for_each(|(i, x)| {
                buffer[i] = x;
            });
        if rlen == 0 {
            Err(vfscore::VfsError::Blocking)
        } else {
            Ok(rlen)
        }
    }

    fn poll(&self, events: PollEvent) -> VfsResult<PollEvent> {
        let mut res = PollEvent::NONE;
        if events.contains(PollEvent::POLLOUT) {
            if self.inner.lock().len() <= 0x50000 {
                res |= PollEvent::POLLOUT;
            }
        }
        if events.contains(PollEvent::POLLIN) {
            if self.inner.lock().len() > 0 {
                res |= PollEvent::POLLIN;
            }
        }
        Ok(res)
    }
}

pub fn create_socket_pair() -> Arc<SocketPair> {
    Arc::new(SocketPair {
        inner: Arc::new(Mutex::new(VecDeque::new()))
    })
}
