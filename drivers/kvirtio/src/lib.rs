#![no_std]
#![feature(used_with_arg)]

extern crate alloc;
#[macro_use]
extern crate log;

pub mod virtio_blk;
pub mod virtio_impl;
pub mod virtio_input;
pub mod virtio_net;

use core::ptr::NonNull;

use alloc::{sync::Arc, vec::Vec};
use arch::VIRT_ADDR_START;
use devices::{
    device::{Driver, UnsupportedDriver},
    driver_define,
};
use fdt::node::FdtNode;
use virtio_drivers::transport::{
    mmio::{MmioTransport, VirtIOHeader},
    DeviceType, Transport,
};

pub fn init_mmio(node: &FdtNode) -> Arc<dyn Driver> {
    if let Some(reg) = node.reg().and_then(|mut reg| reg.next()) {
        let paddr = reg.starting_address as usize;
        let vaddr = VIRT_ADDR_START + paddr;
        let header = NonNull::new(vaddr as *mut VirtIOHeader).unwrap();
        if let Ok(transport) = unsafe { MmioTransport::new(header) } {
            info!(
                "Detected virtio MMIO device with
                    vendor id {:#X}
                    device type {:?}
                    version {:?} 
                    addr @ {:#X} 
                    interrupt: {:?}",
                transport.vendor_id(),
                transport.device_type(),
                transport.version(),
                vaddr,
                node.interrupts().map(|x| x.collect::<Vec<usize>>())
            );
            return virtio_device(transport, node);
        }
    }
    Arc::new(UnsupportedDriver)
}

fn virtio_device(transport: MmioTransport, node: &FdtNode) -> Arc<dyn Driver> {
    match transport.device_type() {
        DeviceType::Block => virtio_blk::init(transport, node),
        DeviceType::Input => virtio_input::init(transport, node),
        DeviceType::Network => virtio_net::init(transport),
        device_type => {
            warn!("Unrecognized virtio device: {:?}", device_type);
            Arc::new(UnsupportedDriver)
        }
    }
}

driver_define!("virtio,mmio", init_mmio);
