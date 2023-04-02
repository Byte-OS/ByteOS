pub mod virtio_blk;
pub mod virtio_impl;

use core::ptr::NonNull;

use arch::VIRT_ADDR_START;
use fdt::node::FdtNode;
use virtio_drivers::transport::{
    mmio::{MmioTransport, VirtIOHeader},
    DeviceType, Transport,
};

use crate::DRIVER_REGS;

pub fn init_mmio(node: &FdtNode) {
    if let Some(reg) = node.reg().and_then(|mut reg| reg.next()) {
        let paddr = reg.starting_address as usize;
        let vaddr = VIRT_ADDR_START + paddr;
        let header = NonNull::new(vaddr as *mut VirtIOHeader).unwrap();
        if let Ok(transport) = unsafe { MmioTransport::new(header) } {
            info!(
                "Detected virtio MMIO device with vendor id {:#X}, device type {:?}, version {:?}",
                transport.vendor_id(),
                transport.device_type(),
                transport.version(),
            );
            virtio_device(transport);
        }
    }
}

fn virtio_device(transport: MmioTransport) {
    match transport.device_type() {
        DeviceType::Block => virtio_blk::init(transport),
        DeviceType::GPU => info!("unsupport gpu device now"),
        DeviceType::Input => info!("unsupport input device now"),
        DeviceType::Network => info!("unsupport net device now"),
        t => warn!("Unrecognized virtio device: {:?}", t),
    }
}

// mmio
pub fn driver_init() {
    DRIVER_REGS.lock().insert("virtio,mmio", init_mmio);
}
