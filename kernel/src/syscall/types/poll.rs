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
        let mut data_map = self.data.lock();
        match ctl {
            EpollCtl::ADD => {
                data_map.insert(fd, ev);
            }
            EpollCtl::DEL => {
                data_map.remove(&fd);
            }
            EpollCtl::MOD => {
                data_map.get_mut(&fd).map(|x| {
                    *x = ev;
                });
            }
        }
    }
}

impl INodeInterface for EpollFile {}
