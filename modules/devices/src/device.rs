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
}

pub trait RtcDriver: Driver {
    fn read_timestamp(&self) -> u64;
}
