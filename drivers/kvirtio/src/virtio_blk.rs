use alloc::sync::Arc;
use alloc::vec::Vec;
use devices::device::{BlkDriver, DeviceWrapperEnum, Driver};
use devices::{node_to_interrupts, register_device_irqs};
use fdt::node::FdtNode;
use sync::Mutex;
use virtio_drivers::device::blk::VirtIOBlk;
use virtio_drivers::transport::mmio::MmioTransport;

use super::virtio_impl::HalImpl;

pub struct VirtIOBlock {
    inner: Mutex<VirtIOBlk<HalImpl, MmioTransport>>,
    irqs: Vec<u32>,
}

unsafe impl Sync for VirtIOBlock {}
unsafe impl Send for VirtIOBlock {}

impl Driver for VirtIOBlock {
    fn interrupts(&self) -> &[u32] {
        &self.irqs
    }

    fn device_type(&self) -> devices::device::DeviceType {
        devices::device::DeviceType::Block
    }

    fn get_id(&self) -> &str {
        "virtio-blk"
    }

    fn get_device_wrapper(self: Arc<Self>) -> DeviceWrapperEnum {
        DeviceWrapperEnum::BLOCK(self.clone())
    }
}

impl BlkDriver for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.inner
            .lock()
            .read_block(block_id, buf)
            .expect("can't read block by virtio block");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.inner
            .lock()
            .write_block(block_id, buf)
            .expect("can't write block by virtio block");
    }
}

pub fn init(transport: MmioTransport, node: &FdtNode) -> Arc<dyn Driver> {
    let blk_device = Arc::new(VirtIOBlock {
        inner: Mutex::new(
            VirtIOBlk::<HalImpl, MmioTransport>::new(transport)
                .expect("failed to create blk driver"),
        ),
        irqs: node_to_interrupts(node),
    });

    register_device_irqs(blk_device.clone());
    info!("Initailize virtio-block device");
    blk_device
}
