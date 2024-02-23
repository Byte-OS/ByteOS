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
pub mod memory;
// pub mod virtio;

use alloc::{collections::BTreeMap, string::String, sync::Arc, vec::Vec};
use device::{BlkDriver, DeviceSet, Driver, IntDriver, NetDriver, UartDriver};
use fdt::{self, node::FdtNode, Fdt};
use kheader::macros::link_define;
use sync::{LazyInit, Mutex};

// pub static DEVICE_TREE_ADDR: AtomicUsize = AtomicUsize::new(0);
pub static DEVICE_TREE: LazyInit<Vec<u8>> = LazyInit::new();
pub static DRIVER_REGS: Mutex<BTreeMap<&str, fn(&FdtNode) -> Arc<dyn Driver>>> =
    Mutex::new(BTreeMap::new());
pub static IRQ_MANAGER: Mutex<BTreeMap<u32, Arc<dyn Driver>>> = Mutex::new(BTreeMap::new());
pub static INT_DEVICE: LazyInit<Arc<dyn IntDriver>> = LazyInit::new();
pub static MAIN_UART: LazyInit<Arc<dyn UartDriver>> = LazyInit::new();
pub static ALL_DEVICES: Mutex<DeviceSet> = Mutex::new(DeviceSet::new());

link_define! {
    pub static DRIVERS_INIT: [fn() -> Option<Arc<dyn Driver>>] = [..];
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
pub fn get_main_uart() -> Arc<dyn UartDriver> {
    MAIN_UART.try_get().expect("can't find main uart").clone()
}

#[inline]
pub fn get_net_device(id: usize) -> Arc<dyn NetDriver> {
    ALL_DEVICES
        .lock()
        .net
        .get(id)
        .expect("can't find int device")
        .clone()
}

pub fn init_device(device_tree: usize) {
    if device_tree == 0 {
        return;
    }
    // DEVICE_TREE_ADDR.store(device_tree, Ordering::Relaxed);
    let fdt = unsafe { Fdt::from_ptr(device_tree as *const u8).unwrap() };
    let mut dt_buf = vec![0u8; fdt.total_size()];
    dt_buf.copy_from_slice(unsafe {
        core::slice::from_raw_parts(device_tree as *const u8, fdt.total_size())
    });
    DEVICE_TREE.init_by(dt_buf);

    // init memory
    // memory::init();
}

/// prepare_drivers
/// This function will init drivers 
#[inline]
pub fn prepare_drivers() {
    let mut all_devices = ALL_DEVICES.lock();
    DRIVERS_INIT.iter().for_each(|f| {
        f().map(|device| all_devices.add_device(device));
    });
}

pub fn try_to_add_device(node: &FdtNode) {
    let mut all_devices = ALL_DEVICES.lock();
    let driver_manager = DRIVER_REGS.lock();
    if let Some(compatible) = node.compatible() {
        info!(
            "    {}  {}",
            node.name,
            compatible.all().intersperse(" ").collect::<String>()
        );
        for compati in compatible.all() {
            if let Some(f) = driver_manager.get(compati) {
                all_devices.add_device(f(&node));
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

pub fn node_to_interrupts(node: &FdtNode) -> Vec<u32> {
    node.interrupts()
        .map(|x| x.map(|x| x as u32).collect())
        .unwrap_or_default()
}

#[macro_export]
macro_rules! driver_define {
    ($body: block) => {
        #[kheader::macros::linker_use($crate::DRIVERS_INIT)]
        #[linkme(crate = kheader::macros::linkme)]
        fn __driver_init() -> Option<alloc::sync::Arc<dyn devices::device::Driver>> {
            $body
        }
    };
    ($obj:expr, $func: expr) => {
        #[kheader::macros::linker_use($crate::DRIVERS_INIT)]
        #[linkme(crate = kheader::macros::linkme)]
        fn __driver_init() -> Option<alloc::sync::Arc<dyn devices::device::Driver>> {
            $crate::DRIVER_REGS.lock().insert($obj, $func);
            None
        }
    };
}
