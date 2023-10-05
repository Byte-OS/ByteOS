#![no_std]
#![feature(used_with_arg)]

extern crate alloc;

use alloc::{sync::Arc, vec::Vec};
use devices::{
    device::{Driver, IntDriver},
    driver_define,
};
use fdt::node::FdtNode;

pub struct PLIC;

impl Driver for PLIC {
    fn device_type(&self) -> devices::device::DeviceType {
        devices::device::DeviceType::Int
    }

    fn get_id(&self) -> &str {
        "riscv-plic"
    }
}

impl IntDriver for PLIC {
    fn register_irq(&self, irq: usize, driver: Arc<dyn Driver>) {
        log::info!("regist a interrupt {} for {}", irq, driver.get_id());
    }
}

pub fn init_driver(_node: &FdtNode) {
    log::info!("Initializing plic driver");
    log::info!("--------------------------------");
    log::info!(
        "interrupts: {:?}",
        _node.interrupts().map(|x| x.collect::<Vec<usize>>())
    );
    log::info!("--------------------------------");
}

driver_define!("sifive,plic-1.0.0", init_driver);
