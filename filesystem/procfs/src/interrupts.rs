use core::cmp;

use alloc::string::String;
use libc_types::types::{Stat, StatMode};
use vfscore::{INodeInterface, VfsResult};

pub struct Interrupts {}

impl Interrupts {
    pub const fn new() -> Self {
        Self {}
    }
}

impl INodeInterface for Interrupts {
    fn readat(&self, offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        let str = String::new();
        // FIXME: Use new interrupts method to record this.
        // for (irq, times) in get_int_records().iter().enumerate() {
        //     if *times == 0 {
        //         continue;
        //     }
        //     str += &format!("{}: {}\r\n", irq, *times);
        // }
        let bytes = str.as_bytes();
        let rsize = cmp::min(bytes.len() - offset, buffer.len());
        buffer[..rsize].copy_from_slice(&bytes[offset..offset + rsize]);
        Ok(rsize)
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
