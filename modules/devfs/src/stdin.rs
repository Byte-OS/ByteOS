use arch::console_getchar;
use log::info;
use vfscore::{INodeInterface, Stat, StatMode, VfsResult};

pub struct Stdin;

impl INodeInterface for Stdin {
    fn read(&self, buffer: &mut [u8]) -> vfscore::VfsResult<usize> {
        info!("buffer len: {}", buffer.len());
        assert!(buffer.len() > 0);
        let mut c = console_getchar() as i8;
        loop {
            if c != -1 {
                break;
            }
            c = console_getchar() as i8;
        }
        buffer[0] = c as u8;
        Ok(1)
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
}
