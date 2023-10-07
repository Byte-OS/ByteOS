#![no_std]
#![feature(used_with_arg)]
#![feature(drain_filter)]
#![feature(decl_macro)]
#![feature(iter_intersperse)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;

pub mod device;
pub mod memory;
// pub mod virtio;

use alloc::{collections::BTreeMap, string::String, sync::Arc, vec::Vec};
use device::{BlkDriver, Driver, InputDriver, IntDriver, NetDriver, RtcDriver};
use fdt::{self, node::FdtNode, Fdt};
use kheader::macros::link_define;
use sync::{LazyInit, Mutex};

// pub static DEVICE_TREE_ADDR: AtomicUsize = AtomicUsize::new(0);
pub static DEVICE_TREE: LazyInit<Vec<u8>> = LazyInit::new();
pub static DRIVER_REGS: Mutex<BTreeMap<&str, fn(&FdtNode) -> Arc<dyn Driver>>> =
    Mutex::new(BTreeMap::new());
pub static IRQ_MANAGER: Mutex<BTreeMap<u32, Arc<dyn Driver>>> = Mutex::new(BTreeMap::new());
pub static RTC_DEVICES: Mutex<Vec<Arc<dyn RtcDriver>>> = Mutex::new(Vec::new());
pub static BLK_DEVICES: Mutex<Vec<Arc<dyn BlkDriver>>> = Mutex::new(Vec::new());
pub static NET_DEVICES: Mutex<Vec<Arc<dyn NetDriver>>> = Mutex::new(Vec::new());
pub static INT_DEVICES: Mutex<Vec<Arc<dyn IntDriver>>> = Mutex::new(Vec::new());
pub static INPUT_DEVICES: Mutex<Vec<Arc<dyn InputDriver>>> = Mutex::new(Vec::new());

link_define! {
    pub static DRIVERS_INIT: [fn() -> Option<Arc<dyn Driver>>] = [..];
}

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

pub fn init_device(device_tree: usize) {
    // DEVICE_TREE_ADDR.store(device_tree, Ordering::Relaxed);
    let fdt = unsafe { Fdt::from_ptr(device_tree as *const u8).unwrap() };
    let mut dt_buf = vec![0u8; fdt.total_size()];
    dt_buf.copy_from_slice(unsafe {
        core::slice::from_raw_parts(device_tree as *const u8, fdt.total_size())
    });
    DEVICE_TREE.init_by(dt_buf);

    // init memory
    memory::init();
}

pub fn add_device(device: Arc<dyn Driver>) {
    // match device.device_type() {
    //     device::DeviceType::Rtc   => RTC_DEVICES.lock().push(device.as_rtc().unwrap()),
    //     device::DeviceType::Block => BLK_DEVICES.lock().push(device.as_blk().unwrap()),
    //     device::DeviceType::Net   => NET_DEVICES.lock().push(device.as_net().unwrap()),
    //     device::DeviceType::Int   => INT_DEVICES.lock().push(device.as_int().unwrap()),
    //     device::DeviceType::Input => INPUT_DEVICES.lock().push(device.as_input().unwrap()),
    //     device::DeviceType::Unsupported => {
    //         log::info!("unsupported device");
    //     },
    // }
    match device.get_device_wrapper() {
        device::DeviceWrapperEnum::RTC(device) => RTC_DEVICES.lock().push(device),
        device::DeviceWrapperEnum::BLOCK(device) => BLK_DEVICES.lock().push(device),
        device::DeviceWrapperEnum::NET(device) => NET_DEVICES.lock().push(device),
        device::DeviceWrapperEnum::INPUT(device) => INPUT_DEVICES.lock().push(device),
        device::DeviceWrapperEnum::INT(device) => INT_DEVICES.lock().push(device),
        device::DeviceWrapperEnum::None => {}
    }
}

pub fn prepare_devices() {
    // let fdt =
    //     unsafe { Fdt::from_ptr(DEVICE_TREE_ADDR.load(Ordering::Acquire) as *const u8).unwrap() };
    let fdt = Fdt::new(DEVICE_TREE.as_ref()).unwrap();
    info!("There has {} CPU(s)", fdt.cpus().count());

    fdt.memory().regions().for_each(|x| {
        info!(
            "memory region {:#X} - {:#X}",
            x.starting_address as usize,
            x.starting_address as usize + x.size.unwrap()
        );
    });

    let node = fdt.all_nodes();

    for f in DRIVERS_INIT {
        f().map(|device| add_device(device));
    }

    let driver_manager = DRIVER_REGS.lock();
    for child in node {
        if let Some(compatible) = child.compatible() {
            info!(
                "    {}  {}",
                child.name,
                compatible.all().intersperse(" ").collect::<String>()
            );
            for compati in compatible.all() {
                if let Some(f) = driver_manager.get(compati) {
                    add_device(f(&child));
                    break;
                }
            }
        }
    }

    debug!("block device len: {}", BLK_DEVICES.lock().len());

    // register the drivers in the IRQ MANAGER.
    if let Some(plic) = INT_DEVICES.lock().first() {
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
