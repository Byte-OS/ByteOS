use alloc::string::String;
use vfscore::{INodeInterface, StatMode, VfsResult};

pub struct Rtc;

pub struct RtcTime {
    sec: u32,
    min: u32,
    hour: u32,
    mday: u32,
    mon: u32,
    year: u32,
    _wday: u32,  // unused
    _yday: u32,  // unused
    _isdst: u32, // unused
}

impl INodeInterface for Rtc {
    fn path(&self) -> VfsResult<String> {
        Ok(String::from("/dev/rtc"))
    }

    fn stat(&self, stat: &mut vfscore::Stat) -> vfscore::VfsResult<()> {
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

    fn ioctl(&self, _command: usize, arg: usize) -> VfsResult<usize> {
        let rtc_time = unsafe { (arg as *mut RtcTime).as_mut().unwrap() };
        rtc_time.sec = 0;
        rtc_time.min = 0;
        rtc_time.hour = 0;
        rtc_time.mday = 0;
        rtc_time.mon = 0;
        rtc_time.year = 0;
        Ok(0)
    }
}
