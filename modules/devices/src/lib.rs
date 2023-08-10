#![no_std]
#![feature(used_with_arg)]
#![feature(drain_filter)]
#![feature(decl_macro)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;

pub mod device;
pub mod memory;
// pub mod virtio;

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use device::{BlkDriver, Driver, NetDriver, RtcDriver};
use fdt::{self, node::FdtNode, Fdt};
use kheader::macros::link_define;
use sync::{LazyInit, Mutex};

// pub static DEVICE_TREE_ADDR: AtomicUsize = AtomicUsize::new(0);
pub static DEVICE_TREE: LazyInit<Vec<u8>> = LazyInit::new();
pub static DRIVER_REGS: Mutex<BTreeMap<&str, fn(&FdtNode)>> = Mutex::new(BTreeMap::new());
pub static RTC_DEVICES: Mutex<Vec<Arc<dyn RtcDriver>>> = Mutex::new(Vec::new());
pub static BLK_DEVICES: Mutex<Vec<Arc<dyn BlkDriver>>> = Mutex::new(Vec::new());
pub static NET_DEVICES: Mutex<Vec<Arc<dyn NetDriver>>> = Mutex::new(Vec::new());

link_define!{
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
        f().map(|device| match device.device_type() {
            device::DeviceType::Rtc => todo!(),
            device::DeviceType::Block => {
                BLK_DEVICES.lock().push(device.as_blk().unwrap());
            }
            device::DeviceType::Net => todo!(),
        });
    }

    let driver_manager = DRIVER_REGS.lock();
    for child in node {
        if let Some(compatible) = child.compatible() {
            if let Some(f) = driver_manager.get(compatible.first()) {
                f(&child);
            }
            info!("    {}  {}", child.name, compatible.first());
        }
    }
}

pub macro driver_define($obj:expr, $body: expr) {
    #[kheader::macros::linker_use($crate::DRIVERS_INIT)]
    #[linkme(crate = kheader::macros::linkme)]
    fn __driver_init() -> Option<Arc<dyn devices::device::Driver>> {
        $body
    }
}
