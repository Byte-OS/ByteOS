use alloc::{sync::Arc, vec::Vec};

use crate::{INT_DEVICE, MAIN_UART};

pub enum DeviceType {
    RTC(Arc<dyn RtcDriver>),
    BLOCK(Arc<dyn BlkDriver>),
    NET(Arc<dyn NetDriver>),
    INPUT(Arc<dyn InputDriver>),
    INT(Arc<dyn IntDriver>),
    UART(Arc<dyn UartDriver>),
    None,
}

pub struct DeviceSet {
    pub rtc: Vec<Arc<dyn RtcDriver>>,
    pub blk: Vec<Arc<dyn BlkDriver>>,
    pub net: Vec<Arc<dyn NetDriver>>,
    pub uart: Vec<Arc<dyn UartDriver>>,
    pub input: Vec<Arc<dyn InputDriver>>,
}

impl DeviceSet {
    pub const fn new() -> Self {
        Self {
            rtc: vec![],
            blk: vec![],
            net: vec![],
            uart: vec![],
            input: vec![],
        }
    }

    pub fn add_device(&mut self, device: Arc<dyn Driver>) {
        match device.get_device_wrapper() {
            DeviceType::RTC(device) => self.rtc.push(device),
            DeviceType::BLOCK(device) => self.blk.push(device),
            DeviceType::NET(device) => self.net.push(device),
            DeviceType::INPUT(device) => self.input.push(device),
            DeviceType::INT(device) => INT_DEVICE.init_by(device),
            DeviceType::UART(device) => {
                if self.uart.len() == 0 {
                    MAIN_UART.init_by(device.clone());
                }
                self.uart.push(device)
            }
            DeviceType::None => {}
        }
    }
}

pub trait Driver: Send + Sync {
    fn get_id(&self) -> &str;

    fn interrupts(&self) -> &[u32] {
        &[]
    }

    fn try_handle_interrupt(&self, _irq: u32) -> bool {
        false
    }

    fn get_device_wrapper(self: Arc<Self>) -> DeviceType;
}

pub trait RtcDriver: Driver {
    fn read_timestamp(&self) -> u64;
    fn read(&self) -> u64;
}

pub trait BlkDriver: Driver {
    fn read_blocks(&self, block_id: usize, buf: &mut [u8]);
    fn write_blocks(&self, block_id: usize, buf: &[u8]);
    fn capacity(&self) -> usize {
        0
    }
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

pub trait UartDriver: Driver {
    fn put(&self, c: u8);
    fn get(&self) -> Option<u8>;
}

pub struct UnsupportedDriver;

impl Driver for UnsupportedDriver {
    fn get_id(&self) -> &str {
        "unsupported-driver"
    }

    fn get_device_wrapper(self: Arc<Self>) -> DeviceType {
        DeviceType::None
    }
}
