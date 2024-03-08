use alloc::sync::Arc;
use alloc::vec::Vec;
use devices::device::{DeviceType, Driver, InputDriver};
use devices::register_device_irqs;
use sync::Mutex;
use virtio_drivers::device::input::VirtIOInput as VirtIOInputWrapper;
use virtio_drivers::transport::mmio::MmioTransport;

use super::virtio_impl::HalImpl;

pub struct VirtIOInput {
    _inner: Mutex<VirtIOInputWrapper<HalImpl, MmioTransport>>,
    interrupts: Vec<u32>,
}

unsafe impl Sync for VirtIOInput {}
unsafe impl Send for VirtIOInput {}

impl Driver for VirtIOInput {
    fn get_id(&self) -> &str {
        "virtio-input"
    }

    fn interrupts(&self) -> &[u32] {
        &self.interrupts
    }

    fn get_device_wrapper(self: Arc<Self>) -> DeviceType {
        DeviceType::INPUT(self.clone())
    }
}

impl InputDriver for VirtIOInput {
    fn read_event(&self) -> u64 {
        todo!()
    }

    fn handle_irq(&self) {
        todo!()
    }

    fn is_empty(&self) -> bool {
        todo!()
    }
}

pub fn init(transport: MmioTransport, irqs: Vec<u32>) -> Arc<dyn Driver> {
    let input_device = Arc::new(VirtIOInput {
        _inner: Mutex::new(
            VirtIOInputWrapper::<HalImpl, MmioTransport>::new(transport)
                .expect("failed to create blk driver"),
        ),
        interrupts: irqs,
    });
    register_device_irqs(input_device.clone());
    info!("Initailize virtio-iput device");
    input_device
}
