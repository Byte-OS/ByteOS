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
use devices::{
    device::{Driver, UnsupportedDriver}, driver_define, fdt::node::FdtNode, node_to_interrupts, VIRT_ADDR_START
};
use virtio_drivers::transport::{
    mmio::{MmioTransport, VirtIOHeader},
    DeviceType, Transport,
};

#[cfg(any(target_arch = "x86_64", target_arch = "loongarch64"))]
use devices::ALL_DEVICES;

#[cfg(any(target_arch = "x86_64", target_arch = "loongarch64"))]
use virtio_drivers::transport::pci::{
    bus::{BarInfo, Cam, Command, DeviceFunction, PciRoot},
    virtio_device_type, PciTransport,
};

#[cfg(any(target_arch = "x86_64", target_arch = "loongarch64"))]
use crate::virtio_impl::HalImpl;

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
    let irqs = node_to_interrupts(node);
    match transport.device_type() {
        DeviceType::Block => virtio_blk::init(transport, irqs),
        DeviceType::Input => virtio_input::init(transport, irqs),
        DeviceType::Network => virtio_net::init(transport, irqs),
        device_type => {
            warn!("Unrecognized virtio device: {:?}", device_type);
            Arc::new(UnsupportedDriver)
        }
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "loongarch64"))]
fn enumerate_pci(mmconfig_base: *mut u8) {
    info!("mmconfig_base = {:#x}", mmconfig_base as usize);

    let mut pci_root = unsafe { PciRoot::new(mmconfig_base, Cam::Ecam) };
    for (device_function, info) in pci_root.enumerate_bus(0) {
        let (status, command) = pci_root.get_status_command(device_function);
        info!(
            "Found {} at {}, status {:?} command {:?}",
            info, device_function, status, command
        );
        if let Some(virtio_type) = virtio_device_type(&info) {
            info!("  VirtIO {:?}", virtio_type);

            // Enable the device to use its BARs.
            pci_root.set_command(
                device_function,
                Command::IO_SPACE | Command::MEMORY_SPACE | Command::BUS_MASTER,
            );
            dump_bar_contents(&mut pci_root, device_function, 4);

            let mut transport =
                PciTransport::new::<HalImpl>(&mut pci_root, device_function).unwrap();
            info!(
                "Detected virtio PCI device with device type {:?}, features {:#018x}",
                transport.device_type(),
                transport.read_device_features(),
            );
            virtio_device_probe(transport);
        }
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "loongarch64"))]
fn virtio_device_probe(transport: impl Transport + 'static) {
    let device = match transport.device_type() {
        DeviceType::Block => Some(virtio_blk::init(transport, Vec::new())),
        // DeviceType::Input => virtio_input::init(transport, Vec::new()),
        DeviceType::Network => Some(virtio_net::init(transport, Vec::new())),
        t => {
            warn!("Unrecognized virtio device: {:?}", t);
            None
        }
    };

    if let Some(device) = device {
        info!("is locked: {}", ALL_DEVICES.is_locked());
        ALL_DEVICES.lock().add_device(device);
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "loongarch64"))]
fn dump_bar_contents(root: &mut PciRoot, device_function: DeviceFunction, bar_index: u8) {
    let bar_info = root.bar_info(device_function, bar_index).unwrap();
    trace!("Dumping bar {}: {:#x?}", bar_index, bar_info);
    if let BarInfo::Memory { address, size, .. } = bar_info {
        let start = address as *const u8;
        unsafe {
            let mut buf = [0u8; 32];
            for i in 0..size / 32 {
                let ptr = start.add(i as usize * 32);
                core::ptr::copy(ptr, buf.as_mut_ptr(), 32);
                if buf.iter().any(|b| *b != 0xff) {
                    trace!("  {:?}: {:x?}", ptr, buf);
                }
            }
        }
    }
    trace!("End of dump");
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "loongarch64")))]
driver_define!("virtio,mmio", init_mmio);

#[cfg(target_arch = "x86_64")]
driver_define!({
    enumerate_pci((0xB000_0000usize | VIRT_ADDR_START) as _);
    None
});

#[cfg(target_arch = "loongarch64")]
driver_define!({
    enumerate_pci((0x2000_0000 | 0x8000000000000000usize) as _);
    None
});
