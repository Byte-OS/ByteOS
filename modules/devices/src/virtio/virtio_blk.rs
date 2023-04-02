use sync::Mutex;
use virtio_drivers::device::blk::VirtIOBlk;
use virtio_drivers::transport::mmio::MmioTransport;

use crate::device::{BlkDriver, Driver};

use super::virtio_impl::HalImpl;

pub struct VirtIOBlock(Mutex<VirtIOBlk<HalImpl, MmioTransport>>);

unsafe impl Sync for VirtIOBlock {}
unsafe impl Send for VirtIOBlock {}

impl Driver for VirtIOBlock {
    fn device_type(&self) -> crate::device::DeviceType {
        crate::device::DeviceType::Block
    }

    fn get_id(&self) -> &str {
        "virtio-blk"
    }

    fn as_blk(&self) -> Option<&dyn crate::device::BlkDriver> {
        Some(self)
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
    let mut input = vec![0xffu8; 512];
    let mut output = vec![0; 512];
    for i in 0..32 {
        for x in input.iter_mut() {
            *x = i as u8;
        }
        blk.write_block(i, &input);
        blk.read_block(i, &mut output);
        assert_eq!(input, output);
    }
    info!("virtio-blk test finished");
}
