use alloc::sync::Arc;
use arch::VIRT_ADDR_START;
use core::ptr::read_volatile;
use fdt::node::FdtNode;
use timestamp::DateTime;

use crate::device::{DeviceType, Driver, RtcDriver};
use crate::{DRIVER_REGS, RTC_DEVICES};

const TIMER_TIME_LOW: usize = 0x00;
const TIMER_TIME_HIGH: usize = 0x04;

pub struct RtcGoldfish {
    base: usize,
}

impl Driver for RtcGoldfish {
    fn device_type(&self) -> DeviceType {
        DeviceType::Rtc
    }

    fn get_id(&self) -> &str {
        "rtc_goldfish"
    }

    fn as_rtc(self: Arc<Self>) -> Option<Arc<dyn RtcDriver>> {
        Some(self.clone())
    }
}

impl RtcDriver for RtcGoldfish {
    // read seconds since 1970-01-01
    fn read_timestamp(&self) -> u64 {
        self.read() / 1_000_000_000u64
    }
    // read value
    #[inline]
    fn read(&self) -> u64 {
        unsafe {
            let low: u32 = read_volatile((self.base + TIMER_TIME_LOW) as *const u32);
            let high: u32 = read_volatile((self.base + TIMER_TIME_HIGH) as *const u32);
            ((high as u64) << 32) | (low as u64)
        }
    }
}

pub fn init_rtc(node: &FdtNode) {
    let addr = node.property("reg").unwrap().value[4..8]
        .iter()
        .fold(0, |acc, x| (acc << 8) | (*x as usize));
    let rtc = Arc::new(RtcGoldfish {
        base: VIRT_ADDR_START + addr,
    });

    RTC_DEVICES.lock().push(rtc.clone());

    let date_time = DateTime::new(rtc.read_timestamp() as usize);

    info!("rtc device initialized.");
    info!(
        "the standard Beijing time: {}   timestamp : {}",
        date_time, date_time.timestamp
    );
}

// 利用 Linkme 初始化
pub fn driver_init() {
    DRIVER_REGS.lock().insert("google,goldfish-rtc", init_rtc);
}
