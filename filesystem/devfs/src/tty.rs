use core::cmp;

use alloc::collections::VecDeque;
use devices::utils::{get_char, puts};
use libc_types::{
    ioctl::TermIoctlCmd,
    poll::PollEvent,
    termios::Termios,
    types::{Stat, StatMode, WinSize},
};
use sync::Mutex;
use syscalls::Errno;
use vfscore::{INodeInterface, VfsResult};
pub struct Tty {
    buffer: Mutex<VecDeque<u8>>,
    termios: Mutex<Termios>,
    pgid: Mutex<u32>,
    winsize: Mutex<WinSize>,
}

impl Tty {
    pub fn new() -> Tty {
        Tty {
            buffer: Mutex::new(VecDeque::new()),
            termios: Default::default(),
            pgid: Default::default(),
            winsize: Mutex::new(WinSize {
                row: 24,
                col: 140,
                xpixel: 0,
                ypixel: 0,
            }),
        }
    }
}

impl INodeInterface for Tty {
    fn readat(&self, _offset: usize, buffer: &mut [u8]) -> vfscore::VfsResult<usize> {
        assert!(buffer.len() > 0);
        let mut self_buffer = self.buffer.lock();
        if self_buffer.len() > 0 {
            let rlen = cmp::min(buffer.len(), self_buffer.len());
            for i in 0..rlen {
                buffer[i] = self_buffer.pop_front().unwrap();
            }
            Ok(rlen)
        } else {
            if let Some(c) = get_char() {
                buffer[0] = c as u8;
                Ok(1)
            } else {
                Err(Errno::EWOULDBLOCK)
            }
        }
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.dev = 1;
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = StatMode::CHAR; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 1000;
        stat.gid = 1000;
        stat.size = 15;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id
        Ok(())
    }

    fn writeat(&self, _offset: usize, buffer: &[u8]) -> vfscore::VfsResult<usize> {
        puts(buffer);
        Ok(buffer.len())
    }

    fn poll(&self, events: PollEvent) -> VfsResult<PollEvent> {
        let mut res = PollEvent::NONE;
        if events.contains(PollEvent::IN) {
            let buf_len = self.buffer.lock().len();
            if buf_len > 0 {
                res |= PollEvent::IN;
            } else {
                if let Some(c) = get_char() {
                    res |= PollEvent::IN;
                    self.buffer.lock().push_back(c);
                }
            }
        }
        if events.contains(PollEvent::OUT) {
            res |= PollEvent::OUT;
        }
        Ok(res)
    }

    fn ioctl(&self, command: usize, arg: usize) -> VfsResult<usize> {
        let cmd = TermIoctlCmd::try_from(command as u32).map_err(|_| Errno::EINVAL)?;
        match cmd {
            TermIoctlCmd::TCGETS | TermIoctlCmd::TCGETA => {
                unsafe {
                    (arg as *mut Termios).write_volatile(*self.termios.lock());
                }
                Ok(0)
            }
            TermIoctlCmd::TCSETS | TermIoctlCmd::TCSETSW | TermIoctlCmd::TCSETSF => {
                // copy_from_user(token, argp as *const Termios, &mut inner.termios);
                unsafe { *self.termios.lock() = *(arg as *mut Termios).as_mut().unwrap() }
                Ok(0)
            }
            TermIoctlCmd::TIOCGPGRP => match unsafe { (arg as *mut u32).as_mut() } {
                Some(pgid) => {
                    *pgid = *self.pgid.lock();
                    Ok(0)
                }
                None => Err(Errno::EINVAL),
            },
            TermIoctlCmd::TIOCSPGRP => match unsafe { (arg as *mut u32).as_mut() } {
                Some(pgid) => {
                    *self.pgid.lock() = *pgid;
                    Ok(0)
                }
                None => Err(Errno::EINVAL),
            },
            TermIoctlCmd::TIOCGWINSZ => {
                unsafe {
                    *(arg as *mut WinSize).as_mut().unwrap() = *self.winsize.lock();
                }
                Ok(0)
            }
            TermIoctlCmd::TIOCSWINSZ => {
                unsafe {
                    *self.winsize.lock() = *(arg as *mut WinSize).as_mut().unwrap();
                }
                Ok(0)
            }
            _ => Err(Errno::EPERM),
        }
        // Err(VfsError::NotSupported)
    }
}
