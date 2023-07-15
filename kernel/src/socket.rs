use core::{cmp, net::SocketAddrV4};

use alloc::{sync::Arc, vec::Vec};
use fs::INodeInterface;
use lose_net_stack::net_trait::SocketInterface;
use sync::Mutex;
use vfscore::{Metadata, PollEvent, VfsResult};

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

#[derive(Clone)]
pub struct SocketOptions {
    pub wsize: usize,
    pub rsize: usize,
}

#[allow(dead_code)]
pub struct Socket {
    pub domain: usize,
    pub net_type: NetType,
    pub inner: Arc<dyn SocketInterface>,
    pub options: Mutex<SocketOptions>,
}

unsafe impl Sync for Socket {}
unsafe impl Send for Socket {}

impl Drop for Socket {
    fn drop(&mut self) {
        log::debug!("strong count: {}", Arc::strong_count(&self.inner));
        // TIPS: the socke table map will consume a strong reference.
        if !self.inner.is_closed().unwrap()
            && (Arc::strong_count(&self.inner) == 2 || Arc::strong_count(&self.inner) == 1)
        {
            // self.inner.close().expect("cant close socket when droping socket in os.");
            self.inner.close();
        }
        // self.inner.close();
    }
}

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
            options: Mutex::new(SocketOptions { wsize: 0, rsize: 0 }),
        })
    }

    pub fn recv_from(&self) -> VfsResult<(Vec<u8>, SocketAddrV4)> {
        log::warn!("try to recv data from {}", self.inner.get_local().unwrap());
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
            options: Mutex::new(SocketOptions { wsize: 0, rsize: 0 }),
        })
    }

    pub fn reuse(&self, port: u16) -> Self {
        // NET_SERVER.get_tcp(port)
        match self.inner.get_protocol().unwrap() {
            lose_net_stack::connection::SocketType::TCP => {
                if let Some(socket_inner) = NET_SERVER.get_tcp(&port) {
                    Self {
                        domain: self.domain,
                        net_type: self.net_type,
                        inner: socket_inner,
                        options: Mutex::new(self.options.lock().clone()),
                    }
                } else {
                    unreachable!("can't reusetcp in blank tcp")
                }
            }
            lose_net_stack::connection::SocketType::UDP => {
                if let Some(socket_inner) = NET_SERVER.get_udp(&port) {
                    Self {
                        domain: self.domain,
                        net_type: self.net_type,
                        inner: socket_inner,
                        options: Mutex::new(self.options.lock().clone()),
                    }
                } else {
                    unreachable!("can't reusetcp in blank udp")
                }
            }
            lose_net_stack::connection::SocketType::RAW => todo!(),
        }
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
                let rlen = cmp::min(data.len(), buffer.len());
                buffer[..rlen].copy_from_slice(&data[..rlen]);
                self.options.lock().rsize += rlen;
                Ok(rlen)
            }
            Err(_err) => Err(vfscore::VfsError::Blocking),
        }
    }

    fn write(&self, buffer: &[u8]) -> VfsResult<usize> {
        match self.inner.sendto(&buffer, None) {
            Ok(len) => {
                self.options.lock().wsize += len;
                Ok(len)
            }
            Err(_err) => Err(vfscore::VfsError::NotWriteable),
        }
    }

    fn poll(&self, events: PollEvent) -> VfsResult<PollEvent> {
        let mut res = PollEvent::NONE;
        if !self.inner.is_closed().unwrap() {
            if events.contains(PollEvent::POLLOUT) {
                res |= PollEvent::POLLOUT;
            }
        }
        if events.contains(PollEvent::POLLIN) {
            if self.inner.readable().unwrap() {
                res |= PollEvent::POLLIN;
            }
        }
        Ok(res)
    }
}
