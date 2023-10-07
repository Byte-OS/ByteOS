use alloc::sync::Arc;

pub enum DeviceType {
    Rtc,
    Block,
    Net,
    Int,
    Input,
    Unsupported,
}

pub enum DeviceWrapperEnum {
    RTC(Arc<dyn RtcDriver>),
    BLOCK(Arc<dyn BlkDriver>),
    NET(Arc<dyn NetDriver>),
    INPUT(Arc<dyn InputDriver>),
    INT(Arc<dyn IntDriver>),
    None,
}

pub trait Driver: Send + Sync {
    fn device_type(&self) -> DeviceType;

    fn get_id(&self) -> &str;

    fn interrupts(&self) -> &[u32] {
        &[]
    }

    fn try_handle_interrupt(&self, _irq: u32) -> bool {
        false
    }

    fn get_device_wrapper(self: Arc<Self>) -> DeviceWrapperEnum;
}

pub trait RtcDriver: Driver {
    fn read_timestamp(&self) -> u64;
    fn read(&self) -> u64;
}

pub trait BlkDriver: Driver {
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    fn write_block(&self, block_id: usize, buf: &[u8]);
}

#[derive(Debug)]
pub enum NetError {
    NoData,
}

pub trait NetDriver: Driver {
    fn recv(&self, buf: &mut [u8]) -> Result<usize, NetError>;
    fn send(&self, buf: &[u8]) -> Result<(), NetError>;
}

pub trait IntDriver: Driver {
    fn register_irq(&self, irq: u32, driver: Arc<dyn Driver>);
}

pub trait InputDriver: Driver {
    fn read_event(&self) -> u64;
    fn handle_irq(&self);
    fn is_empty(&self) -> bool;
}

pub struct UnsupportedDriver;

impl Driver for UnsupportedDriver {
    fn device_type(&self) -> DeviceType {
        DeviceType::Unsupported
    }

    fn get_id(&self) -> &str {
        "unsupported-driver"
    }

    fn get_device_wrapper(self: Arc<Self>) -> DeviceWrapperEnum {
        DeviceWrapperEnum::None
    }
}
