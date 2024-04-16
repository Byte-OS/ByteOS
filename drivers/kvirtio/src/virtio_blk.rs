use alloc::sync::Arc;
use alloc::vec::Vec;
use devices::device::{BlkDriver, DeviceType, Driver};
use devices::{register_device_irqs, Mutex};
use virtio_drivers::device::blk::VirtIOBlk;
use virtio_drivers::transport::Transport;

use super::virtio_impl::HalImpl;

pub struct VirtIOBlock<T: Transport> {
    inner: Mutex<VirtIOBlk<HalImpl, T>>,
    irqs: Vec<u32>,
}

unsafe impl<T: Transport> Sync for VirtIOBlock<T> {}
unsafe impl<T: Transport> Send for VirtIOBlock<T> {}

impl<T: Transport + 'static> Driver for VirtIOBlock<T> {
    fn interrupts(&self) -> &[u32] {
        &self.irqs
    }

    fn get_id(&self) -> &str {
        "virtio-blk"
    }

    fn get_device_wrapper(self: Arc<Self>) -> DeviceType {
        DeviceType::BLOCK(self.clone())
    }
}

impl<T: Transport + 'static> BlkDriver for VirtIOBlock<T> {
    fn read_blocks(&self, block_id: usize, buf: &mut [u8]) {
        self.inner
            .lock()
            .read_blocks(block_id, buf)
            .expect("can't read block by virtio block");
    }

    fn write_blocks(&self, block_id: usize, buf: &[u8]) {
        self.inner
            .lock()
            .write_blocks(block_id, buf)
            .expect("can't write block by virtio block");
    }
}

pub fn init<T: Transport + 'static>(transport: T, irqs: Vec<u32>) -> Arc<dyn Driver> {
    let blk_device = Arc::new(VirtIOBlock {
        inner: Mutex::new(
            VirtIOBlk::<HalImpl, T>::new(transport).expect("failed to create blk driver"),
        ),
        irqs,
    });

    register_device_irqs(blk_device.clone());
    info!("Initailize virtio-block device");
    blk_device
}
