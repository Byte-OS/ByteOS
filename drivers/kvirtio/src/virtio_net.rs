use core::cmp;

use alloc::sync::Arc;
use alloc::vec::Vec;
use devices::device::{DeviceType, Driver, NetDriver, NetError};
use devices::{register_device_irqs, Mutex};
use virtio_drivers::device::net::{self, TxBuffer};
use virtio_drivers::transport::Transport;

use super::virtio_impl::HalImpl;

#[allow(dead_code)]
pub struct VirtIONet<T: Transport> {
    inner: Mutex<net::VirtIONet<HalImpl, T, 32>>,
    irqs: Vec<u32>,
}

unsafe impl<T: Transport> Sync for VirtIONet<T> {}
unsafe impl<T: Transport> Send for VirtIONet<T> {}

impl<T: Transport + 'static> Driver for VirtIONet<T> {
    fn get_id(&self) -> &str {
        "virtio-blk"
    }

    fn get_device_wrapper(self: Arc<Self>) -> DeviceType {
        DeviceType::NET(self.clone())
    }
}

impl<T: Transport + 'static> NetDriver for VirtIONet<T> {
    fn recv(&self, buf: &mut [u8]) -> Result<usize, NetError> {
        let packet = self.inner.lock().receive().map_err(|_| NetError::NoData)?;
        let rlen = cmp::min(buf.len(), packet.packet_len());
        buf[..rlen].copy_from_slice(&packet.packet()[..rlen]);
        self.inner
            .lock()
            .recycle_rx_buffer(packet)
            .expect("can't receive data");
        Ok(rlen)
    }

    fn send(&self, buf: &[u8]) -> Result<(), NetError> {
        self.inner
            .lock()
            .send(TxBuffer::from(buf))
            .expect("can't send data");
        Ok(())
    }
}

pub fn init<T: Transport + 'static>(transport: T, irqs: Vec<u32>) -> Arc<dyn Driver> {
    info!("Initailize virtio-net device, irqs: {:?}", irqs);
    let net_device = Arc::new(VirtIONet {
        inner: Mutex::new(
            net::VirtIONet::<HalImpl, T, 32>::new(transport, 2048)
                .expect("failed to create blk driver"),
        ),
        irqs,
    });
    register_device_irqs(net_device.clone());
    net_device
}
