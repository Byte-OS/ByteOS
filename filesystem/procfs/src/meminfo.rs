use libc_types::types::{Stat, StatMode};
use vfscore::{INodeInterface, VfsResult};

pub struct MemInfo {}

impl MemInfo {
    pub const fn new() -> Self {
        Self {}
    }
}

impl INodeInterface for MemInfo {
    fn readat(&self, _offset: usize, _buffer: &mut [u8]) -> VfsResult<usize> {
        Ok(0)
    }

    fn stat(&self, stat: &mut Stat) -> vfscore::VfsResult<()> {
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
