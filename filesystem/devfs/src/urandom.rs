use vfscore::{INodeInterface, PollEvent, Stat, StatMode, VfsResult};

pub struct Urandom;

impl INodeInterface for Urandom {
    fn readat(&self, _offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        buffer.fill(0);
        Ok(buffer.len())
    }

    fn writeat(&self, _offset: usize, buffer: &[u8]) -> VfsResult<usize> {
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

    fn poll(&self, events: vfscore::PollEvent) -> VfsResult<vfscore::PollEvent> {
        let mut res = PollEvent::NONE;
        if events.contains(PollEvent::POLLIN) {
            res |= PollEvent::POLLIN;
        }
        Ok(res)
    }
}
