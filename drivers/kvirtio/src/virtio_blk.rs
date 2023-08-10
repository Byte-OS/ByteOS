use alloc::sync::Arc;
use devices::BLK_DEVICES;
use devices::device::{Driver, BlkDriver};
use sync::Mutex;
use virtio_drivers::device::blk::VirtIOBlk;
use virtio_drivers::transport::mmio::MmioTransport;

use super::virtio_impl::HalImpl;

pub struct VirtIOBlock(Mutex<VirtIOBlk<HalImpl, MmioTransport>>);

unsafe impl Sync for VirtIOBlock {}
unsafe impl Send for VirtIOBlock {}

impl Driver for VirtIOBlock {
    fn device_type(&self) -> devices::device::DeviceType {
        devices::device::DeviceType::Block
    }

    fn get_id(&self) -> &str {
        "virtio-blk"
    }

    fn as_blk(self: Arc<Self>) -> Option<Arc<dyn BlkDriver>> {
        Some(self.clone())
    }
}

impl BlkDriver for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .lock()
            .read_block(block_id, buf)
            .expect("can't read block by virtio block");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .lock()
            .write_block(block_id, buf)
            .expect("can't write block by virtio block");
    }
}

pub fn init(transport: MmioTransport) {
    let blk = VirtIOBlock(Mutex::new(
        VirtIOBlk::<HalImpl, MmioTransport>::new(transport).expect("failed to create blk driver"),
    ));
    BLK_DEVICES.lock().push(Arc::new(blk));
    info!("Initailize virtio-block device");
}
