#![no_std]
#![feature(used_with_arg)]
#![feature(decl_macro)]
#![feature(iter_intersperse)]
#![feature(fn_ptr_trait)]

extern crate log;
#[macro_use]
extern crate alloc;

pub mod device;
pub mod ipc_uart;
pub mod utils;

use alloc::{sync::Arc, vec::Vec};
use device::{BlkDriver, DeviceSet, Driver, IntDriver, NetDriver, UartDriver};
pub use linkme::{self, distributed_slice as linker_use};
pub use sync::{LazyInit, Mutex, MutexGuard};

pub static INT_DEVICE: LazyInit<Arc<dyn IntDriver>> = LazyInit::new();
pub static MAIN_UART: LazyInit<Arc<dyn UartDriver>> = LazyInit::new();
pub static ALL_DEVICES: Mutex<DeviceSet> = Mutex::new(DeviceSet::new());

#[linkme::distributed_slice]
pub static DRIVERS_INIT: [fn() -> Option<Arc<dyn Driver>>] = [..];

#[macro_export]
macro_rules! driver_define {
    ($body: block) => {
        #[devices::linker_use($crate::DRIVERS_INIT)]
        #[linkme(crate = devices::linkme)]
        fn __driver_init() -> Option<alloc::sync::Arc<dyn devices::device::Driver>> {
            $body
        }
    };
    ($obj:expr, $func: expr) => {
        #[devices::linker_use($crate::DRIVERS_INIT)]
        #[linkme(crate = devices::linkme)]
        fn __driver_init() -> Option<alloc::sync::Arc<dyn devices::device::Driver>> {
            $crate::DRIVER_REGS.lock().insert($obj, $func);
            None
        }
    };
}

#[inline]
pub fn get_blk_device(id: usize) -> Option<Arc<dyn BlkDriver>> {
    let all_device = ALL_DEVICES.lock();
    let len = all_device.blk.len();
    match id < len {
        true => Some(all_device.blk[id].clone()),
        false => None,
    }
}

#[inline]
pub fn get_blk_devices() -> Vec<Arc<dyn BlkDriver>> {
    ALL_DEVICES.lock().blk.clone()
}

#[inline]
pub fn get_int_device() -> Arc<dyn IntDriver> {
    INT_DEVICE.try_get().expect("can't find int device").clone()
}

#[inline]
pub fn get_main_uart() -> Option<Arc<dyn UartDriver>> {
    MAIN_UART.try_get().cloned()
}

#[inline]
pub fn get_net_device(id: usize) -> Arc<dyn NetDriver> {
    ALL_DEVICES
        .lock()
        .net
        .get(id)
        .expect("can't find net device")
        .clone()
}

/// prepare_drivers
/// This function will init drivers
#[inline]
pub fn prepare_drivers() {
    DRIVERS_INIT.iter().for_each(|f| {
        f().map(|device| {
            log::debug!("init driver: {}", device.get_id());
            ALL_DEVICES.lock().add_device(device);
        });
    });
}
