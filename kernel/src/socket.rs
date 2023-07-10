use core::{cmp, net::SocketAddrV4};

use alloc::{sync::Arc, vec::Vec};
use fs::INodeInterface;

use lose_net_stack::net_trait::SocketInterface;
use vfscore::{Metadata, VfsResult, PollEvent};

use crate::syscall::NET_SERVER;

#[derive(Clone, Copy, PartialEq, Debug)]
#[allow(dead_code)]
pub enum NetType {
    STEAM,
    DGRAME,
    RAW,
}

impl NetType {
    pub fn from_usize(value: usize) -> Option<Self> {
        match value {
            1 => Some(Self::STEAM),
            2 => Some(Self::DGRAME),
            _ => None,
        }
    }
}

#[allow(dead_code)]
pub struct Socket {
    pub domain: usize,
    pub net_type: NetType,
    pub inner: Arc<dyn SocketInterface>,
}

unsafe impl Sync for Socket {}
unsafe impl Send for Socket {}

impl Socket {
    pub fn new(domain: usize, net_type: NetType) -> Arc<Self> {
        let inner: Arc<dyn SocketInterface> = match net_type {
            NetType::STEAM => NET_SERVER.blank_tcp(),
            NetType::DGRAME => NET_SERVER.blank_udp(),
            NetType::RAW => {
                panic!("can't create raw socket")
            }
        };
        Arc::new(Self {
            domain,
            net_type,
            inner,
        })
    }

    pub fn recv_from(&self) -> VfsResult<(Vec<u8>, SocketAddrV4)> {
        match self.inner.recv_from() {
            Ok((data, remote)) => Ok((data, remote)),
            Err(_err) => Err(vfscore::VfsError::Blocking),
        }
    }

    pub fn new_with_inner(
        domain: usize,
        net_type: NetType,
        inner: Arc<dyn SocketInterface>,
    ) -> Arc<Self> {
        Arc::new(Self {
            domain,
            net_type,
            inner,
        })
    }
}

impl INodeInterface for Socket {
    fn metadata(&self) -> VfsResult<Metadata> {
        Ok(Metadata {
            filename: "",
            inode: 0,
            file_type: vfscore::FileType::Socket,
            size: 0,
            childrens: 0,
        })
    }

    fn read(&self, buffer: &mut [u8]) -> VfsResult<usize> {
        match self.inner.recv_from() {
            Ok((data, _)) => {
                let wlen = cmp::min(data.len(), buffer.len());
                buffer[..wlen].copy_from_slice(&data[..wlen]);
                Ok(wlen)
            }
            Err(_err) => Err(vfscore::VfsError::Blocking),
        }
    }

    fn write(&self, buffer: &[u8]) -> VfsResult<usize> {
        match self.inner.sendto(&buffer, None) {
            Ok(len) => Ok(len),
            Err(_err) => Err(vfscore::VfsError::NotWriteable),
        }
    }

    fn poll(&self, events: PollEvent) -> VfsResult<PollEvent> {
        let mut res = PollEvent::NONE;
        if !self.inner.is_closed().unwrap() {
            if events.contains(PollEvent::POLLIN) {
                if self.inner.readable().unwrap() {
                    res |= PollEvent::POLLIN;
                }
            }
            if events.contains(PollEvent::POLLOUT) {
                res |= PollEvent::POLLOUT;
            }
        }
        Ok(res)
    }

}
