use hashbrown::HashMap;
use num_derive::FromPrimitive;
use sync::Mutex;
use vfscore::{INodeInterface, PollEvent};

#[repr(C)]
#[derive(Clone, Debug)]
pub struct EpollEvent {
    pub events: EpollEventType,
    pub data: u64,
}

bitflags! {
    /// Epoll Event Type, it is similar as the PollEvent type.
    #[derive(Clone, Debug)]
    pub struct EpollEventType: u32 {
        const EPOLLIN = 0x001;
        const EPOLLOUT = 0x004;
        const EPOLLERR = 0x008;
        const EPOLLHUP = 0x010;
        const EPOLLPRI = 0x002;
        const EPOLLRDNORM = 0x040;
        const EPOLLRDBAND = 0x080;
        const EPOLLWRNORM = 0x100;
        const EPOLLWRBAND= 0x200;
        const EPOLLMSG = 0x400;
        const EPOLLRDHUP = 0x2000;
        const EPOLLEXCLUSIVE = 0x1000_0000;
        const EPOLLWAKEUP = 0x2000_0000;
        const EPOLLONESHOT = 0x4000_0000;
        const EPOLLET = 0x8000_0000;

    }
}

impl EpollEventType {
    pub fn to_poll(&self) -> PollEvent {
        PollEvent::from_bits_truncate(self.bits() as u16)
    }
}

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

#[repr(u8)]
#[derive(Debug, Eq, PartialEq, FromPrimitive)]
/// epoll_ctl
pub enum EpollCtl {
    ADD = 1,
    DEL = 2,
    MOD = 3,
}
