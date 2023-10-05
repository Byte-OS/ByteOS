use alloc::sync::Arc;

pub enum DeviceType {
    Rtc,
    Block,
    Net,
    Int,
}

pub trait Driver: Sync + Send {
    fn device_type(&self) -> DeviceType;

    fn get_id(&self) -> &str;

    fn interrupts(&self) -> &[u32] {
        &[]
    }

    fn try_handle_interrupt(&self, _irq: u32) -> bool {
        false
    }

    fn as_rtc(self: Arc<Self>) -> Option<Arc<dyn RtcDriver>> {
        None
    }

    fn as_blk(self: Arc<Self>) -> Option<Arc<dyn BlkDriver>> {
        None
    }

    fn as_net(self: Arc<Self>) -> Option<Arc<dyn NetDriver>> {
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

#[derive(Debug)]
pub enum NetError {
    NoData,
}

pub trait NetDriver: Driver {
    fn recv(&self, buf: &mut [u8]) -> Result<usize, NetError>;
    fn send(&self, buf: &[u8]) -> Result<(), NetError>;
}

pub trait IntDriver: Driver {
    fn register_irq(&self, irq: usize, driver: Arc<dyn Driver>);
}

pub trait InputDevice: Driver{
    fn read_event(&self) -> u64;
    fn handle_irq(&self);
    fn is_empty(&self) -> bool;
}