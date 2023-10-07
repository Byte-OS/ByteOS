#![no_std]
#![feature(used_with_arg)]

extern crate alloc;

mod plic;

use alloc::sync::Arc;
use arch::{enable_external_irq, VIRT_ADDR_START};
use devices::{
    device::{DeviceWrapperEnum, Driver, IntDriver},
    driver_define,
};
use fdt::node::FdtNode;

pub struct PLIC {
    base: usize,
}

impl Driver for PLIC {
    fn device_type(&self) -> devices::device::DeviceType {
        devices::device::DeviceType::Int
    }

    fn get_id(&self) -> &str {
        "riscv-plic"
    }

    fn try_handle_interrupt(&self, _irq: u32) -> bool {
        let claim = self.get_irq_claim(0, true);
        self.complete_irq_claim(0, true, claim);
        false
    }

    fn get_device_wrapper(self: Arc<Self>) -> DeviceWrapperEnum {
        DeviceWrapperEnum::INT(self.clone())
    }
}

impl IntDriver for PLIC {
    fn register_irq(&self, irq: u32, driver: Arc<dyn Driver>) {
        log::info!("regist a interrupt {} for {}", irq, driver.get_id());
        self.set_irq_enable(0, true, irq);
        self.set_priority(irq, 7);
    }
}

pub fn init_driver(node: &FdtNode) -> Arc<dyn Driver> {
    let addr = node.property("reg").unwrap().value[4..8]
        .iter()
        .fold(0, |acc, x: &u8| (acc << 8) | (*x as usize));
    let plic = Arc::new(PLIC {
        base: VIRT_ADDR_START + addr,
    });
    plic.set_thresold(0, true, 0);
    enable_external_irq();
    plic
}

driver_define!("riscv,plic0", init_driver);
