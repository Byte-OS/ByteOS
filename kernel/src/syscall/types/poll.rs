use hashbrown::HashMap;
use libc_types::epoll::{EpollCtl, EpollEvent};
use sync::Mutex;
use vfscore::INodeInterface;

#[derive(Debug)]
pub struct EpollFile {
    pub data: Mutex<HashMap<usize, EpollEvent>>,
    pub flags: usize,
}

impl EpollFile {
    pub fn new(flags: usize) -> Self {
        EpollFile {
            data: Mutex::new(HashMap::new()),
            flags,
        }
    }

    pub fn ctl(&self, ctl: EpollCtl, fd: usize, ev: EpollEvent) {
        match ctl {
            EpollCtl::ADD => {
                self.data.lock().insert(fd, ev);
            }
            EpollCtl::DEL => {
                self.data.lock().remove(&fd);
            }
            EpollCtl::MOD => {
                self.data.lock().get_mut(&fd).map(|x| {
                    *x = ev;
                });
            }
        }
    }
}

impl INodeInterface for EpollFile {}
