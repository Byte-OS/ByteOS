#![no_std]
#![feature(used_with_arg)]
#![feature(drain_filter)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;

pub mod device;
pub mod memory;
#[cfg(feature = "nvme")]
pub mod nvme;
pub mod rtc;
pub mod virtio;

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicUsize, Ordering};
use device::{BlkDriver, RtcDriver};
use fdt::{self, node::FdtNode, Fdt};
use sync::Mutex;

pub static DEVICE_TREE_ADDR: AtomicUsize = AtomicUsize::new(0);
pub static DRIVER_REGS: Mutex<BTreeMap<&str, fn(&FdtNode)>> = Mutex::new(BTreeMap::new());
pub static RTC_DEVICES: Mutex<Vec<Arc<dyn RtcDriver>>> = Mutex::new(Vec::new());
pub static BLK_DEVICES: Mutex<Vec<Arc<dyn BlkDriver>>> = Mutex::new(Vec::new());

pub fn get_blk_device(id: usize) -> Option<Arc<dyn BlkDriver>> {
    let len = BLK_DEVICES.lock().len();
    match id < len {
        true => Some(BLK_DEVICES.lock()[id].clone()),
        false => None,
    }
}

pub fn get_blk_devices() -> Vec<Arc<dyn BlkDriver>> {
    return BLK_DEVICES.lock().clone();
}

pub fn init_drivers() {
    virtio::driver_init();
    rtc::driver_init();
}

pub fn init_device(device_tree: usize) {
    // 初始化所有驱动
    init_drivers();

    DEVICE_TREE_ADDR.store(device_tree, Ordering::Relaxed);

    // init memory
    memory::init();
}

pub fn prepare_devices() {
    let fdt =
        unsafe { Fdt::from_ptr(DEVICE_TREE_ADDR.load(Ordering::Acquire) as *const u8).unwrap() };
    info!("There has {} CPU(s)", fdt.cpus().count());

    fdt.memory().regions().for_each(|x| {
        info!(
            "memory region {:#X} - {:#X}",
            x.starting_address as usize,
            x.starting_address as usize + x.size.unwrap()
        );
    });

    let node = fdt.all_nodes();

    let driver_manager = DRIVER_REGS.lock();
    for child in node {
        if let Some(compatible) = child.compatible() {
            if let Some(f) = driver_manager.get(compatible.first()) {
                f(&child);
            }
            // info!("    {}  {}", child.name, compatible.first());
        }
    }
    #[cfg(feature = "nvme")]
    nvme::init();
}
