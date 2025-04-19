#![no_std]
#![feature(used_with_arg)]
#![feature(decl_macro)]
#![feature(iter_intersperse)]
#![feature(fn_ptr_trait)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;

pub mod device;
pub mod utils;

pub use fdt_parser as fdt;

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use device::{BlkDriver, DeviceSet, Driver, IntDriver, NetDriver, UartDriver};
use fdt_parser::Node;
pub use linkme::{self, distributed_slice as linker_use};
pub use polyhal::{consts::VIRT_ADDR_START, pagetable::PAGE_SIZE};
pub use runtime::frame::{frame_alloc, frame_alloc_much, FrameTracker};
pub use sync::{LazyInit, Mutex, MutexGuard};

pub static DEVICE_TREE: LazyInit<Vec<u8>> = LazyInit::new();
pub static DRIVER_REGS: Mutex<BTreeMap<&str, fn(&Node) -> Arc<dyn Driver>>> =
    Mutex::new(BTreeMap::new());
pub static IRQ_MANAGER: Mutex<BTreeMap<u32, Arc<dyn Driver>>> = Mutex::new(BTreeMap::new());
pub static INT_DEVICE: LazyInit<Arc<dyn IntDriver>> = LazyInit::new();
pub static MAIN_UART: LazyInit<Arc<dyn UartDriver>> = LazyInit::new();
pub static ALL_DEVICES: Mutex<DeviceSet> = Mutex::new(DeviceSet::new());

#[linkme::distributed_slice]
pub static DRIVERS_INIT: [fn() -> Option<Arc<dyn Driver>>] = [..];

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

pub fn try_to_add_device(node: &Node) {
    let driver_manager = DRIVER_REGS.lock();
    if let Some(mut compatible) = node.compatible() {
        info!(
            "    {}  {:?}",
            node.name,
            compatible.next() // compatible.intersperse(" ").collect::<String>()
        );
        for compati in node.compatibles() {
            if let Some(f) = driver_manager.get(compati) {
                ALL_DEVICES.lock().add_device(f(&node));
                break;
            }
        }
    }
}

pub fn regist_devices_irq() {
    // register the drivers in the IRQ MANAGER.
    if let Some(plic) = INT_DEVICE.try_get() {
        for (irq, driver) in IRQ_MANAGER.lock().iter() {
            plic.register_irq(*irq, driver.clone());
        }
    }
}

// register the irqs
pub fn register_device_irqs(driver: Arc<dyn Driver>) {
    let mut irq_manager = IRQ_MANAGER.lock();
    driver.interrupts().iter().for_each(|irq| {
        irq_manager.insert(*irq, driver.clone());
    });
}

pub fn node_to_interrupts(node: &Node) -> Vec<u32> {
    node.interrupts()
        .map(|x| x.flatten().collect())
        .unwrap_or_default()
}

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
