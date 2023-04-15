pub enum DeviceType {
    Rtc,
    Block,
    Net,
}

pub trait Driver: Sync + Send {
    fn device_type(&self) -> DeviceType;

    fn get_id(&self) -> &str;

    fn as_rtc(&self) -> Option<&dyn RtcDriver> {
        None
    }

    fn as_blk(&self) -> Option<&dyn BlkDriver> {
        None
    }
}

pub trait RtcDriver: Driver {
    fn read_timestamp(&self) -> u64;
    fn read(&self) -> u64;
}

pub trait BlkDriver: Driver {
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    fn write_block(&self, block_id: usize, buf: &[u8]);
}
