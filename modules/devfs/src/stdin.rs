use core::cmp;

use alloc::collections::VecDeque;
use arch::console_getchar;
use sync::Mutex;
use vfscore::{INodeInterface, PollEvent, Stat, StatMode, VfsResult};

pub struct Stdin {
    buffer: Mutex<VecDeque<u8>>,
}

impl Stdin {
    pub fn new() -> Stdin {
        Stdin {
            buffer: Mutex::new(VecDeque::new()),
        }
    }
}

impl INodeInterface for Stdin {
    fn read(&self, buffer: &mut [u8]) -> vfscore::VfsResult<usize> {
        assert!(buffer.len() > 0);
        let mut c = console_getchar() as u8;
        let mut self_buffer = self.buffer.lock();
        if self_buffer.len() > 0 {
            let rlen = cmp::min(buffer.len(), self_buffer.len());
            for i in 0..rlen {
                buffer[i] = self_buffer.pop_front().unwrap();
            }
            Ok(rlen)
        } else {
            loop {
                if c != (-1 as i8 as u8) {
                    break;
                }
                c = console_getchar() as u8;
            }
            buffer[0] = c as u8;
            Ok(1)
        }
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.dev = 0;
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = StatMode::CHAR; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 1000;
        stat.gid = 1000;
        stat.size = 0;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id
        Ok(())
    }

    fn poll(&self, _events: PollEvent) -> VfsResult<PollEvent> {
        todo!()
    }
}
