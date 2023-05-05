use logging::puts;
use vfscore::{INodeInterface, PollEvent, Stat, StatMode, VfsResult};

pub struct Stdout;

impl INodeInterface for Stdout {
    fn write(&self, buffer: &[u8]) -> vfscore::VfsResult<usize> {
        puts(buffer);
        Ok(buffer.len())
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

    fn poll(&self, _events: vfscore::PollEvent) -> VfsResult<vfscore::PollEvent> {
        Ok(PollEvent::POLLOUT)
    }
}
