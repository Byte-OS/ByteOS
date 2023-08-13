use core::cmp;

use alloc::sync::Arc;
use devices::device::{DeviceType, Driver, NetDriver, NetError};
use devices::NET_DEVICES;
use sync::Mutex;
use virtio_drivers::device::net::{self, TxBuffer};
use virtio_drivers::transport::mmio::MmioTransport;

use super::virtio_impl::HalImpl;

pub struct VirtIONet(Mutex<net::VirtIONet<HalImpl, MmioTransport, 32>>);

unsafe impl Sync for VirtIONet {}
unsafe impl Send for VirtIONet {}

impl Driver for VirtIONet {
    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    fn get_id(&self) -> &str {
        "virtio-blk"
    }

    fn as_net(self: Arc<Self>) -> Option<Arc<dyn NetDriver>> {
        Some(self)
    }
}

impl NetDriver for VirtIONet {
    fn recv(&self, buf: &mut [u8]) -> Result<usize, NetError> {
        let packet = self.0.lock().receive().map_err(|_| NetError::NoData)?;
        let rlen = cmp::min(buf.len(), packet.packet_len());
        buf[..rlen].copy_from_slice(&packet.packet()[..rlen]);
        self.0
            .lock()
            .recycle_rx_buffer(packet)
            .expect("can't receive data");
        Ok(rlen)
    }

    fn send(&self, buf: &[u8]) -> Result<(), NetError> {
        self.0
            .lock()
            .send(TxBuffer::from(buf))
            .expect("can't send data");
        Ok(())
    }
}

pub fn init(transport: MmioTransport) {
    let net = VirtIONet(Mutex::new(
        net::VirtIONet::<HalImpl, MmioTransport, 32>::new(transport, 2048)
            .expect("failed to create blk driver"),
    ));
    NET_DEVICES.lock().push(Arc::new(net));
    info!("Initailize virtio-net device");
}
