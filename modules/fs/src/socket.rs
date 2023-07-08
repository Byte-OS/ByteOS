

use alloc::sync::Arc;
use lose_net_stack::{connection::{udp::UdpServer, tcp::{TcpServer, TcpConnection}}, net_trait::NetInterface};
use sync::LazyInit;
use vfscore::{INodeInterface, Metadata, VfsResult};

#[derive(Clone, Copy, PartialEq, Debug)]
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

pub enum SocketWrapper<T: NetInterface> {
    Udp(Arc<UdpServer<T>>),
    TcpServer(Arc<TcpServer<T>>),
    TcpConnection(Arc<TcpConnection<T>>)
}

unsafe impl<T: NetInterface> Sync for SocketWrapper<T> {}
unsafe impl<T: NetInterface> Send for SocketWrapper<T> {}

#[allow(dead_code)]
pub struct Socket<T: NetInterface> {
    pub domain: usize,
    pub net_type: NetType,
    // pub inner: Option<SocketWrapper<T>>,
    pub inner: LazyInit<SocketWrapper<T>>
}

impl<T: NetInterface + 'static> INodeInterface for Socket<T> {
    fn metadata(&self) -> VfsResult<Metadata> {
        Ok(Metadata {
            filename: "",
            inode: 0,
            file_type: vfscore::FileType::Socket,
            size: 0,
            childrens: 0,
        })
    }

    // fn read(&self, buffer: &mut [u8]) -> VfsResult<usize> {
    //     let mut inner = self.inner.lock();
    //     if inner.datas.len() == 0 {
    //         return Ok(0);
    //     }
    //     let rlen = cmp::min(buffer.len(), inner.datas[0].len());
    //     buffer[..rlen].copy_from_slice(inner.datas[0].drain(..rlen).as_slice());
    //     if inner.datas[0].len() == 0 {
    //         inner.datas.pop_front();
    //     }

    //     Ok(rlen)
    // }

    // fn write(&self, buffer: &[u8]) -> VfsResult<usize> {
    //     let wlen = buffer.len();
    //     let inner = self.inner.lock();
    //     match self.net_type {
    //         NetType::STEAM => {
    //             T::tcp_send(
    //                 inner.target_ip,
    //                 inner.target_port,
    //                 inner.ack,
    //                 inner.seq,
    //                 inner.flags,
    //                 inner.win,
    //                 inner.urg,
    //                 buffer,
    //             );
    //         }
    //         NetType::DGRAME => {
    //             T::udp_send(inner.target_ip, inner.target_port, buffer);
    //         }
    //         NetType::RAW => todo!(),
    //     }
    //     Ok(wlen)
    // }
}

impl<T: NetInterface> Socket<T> {
    pub fn new(domain: usize, net_type: NetType) -> Arc<Self> {
        Arc::new(Self {
            domain,
            net_type,
            inner: LazyInit::new()
        })
    }

    pub fn bind(&self, port: u16) {
        // self.inner.lock().port = port;
    }

    pub fn listen(&self) {
        // self.inner.lock().listened = true;
    }

    pub fn add_socket(&self, child: Arc<Socket<T>>) {
        // let mut inner = self.inner.lock();
        // inner.clients.drain_filter(|x| x.upgrade().is_none());
        // inner.clients.push(Arc::downgrade(&child));
    }

    pub fn conn_num(&self) -> usize {
        todo!()
        // let mut inner = self.inner.lock();
        // inner.clients.drain_filter(|x| x.upgrade().is_none());
        // inner.clients.len()
    }

    pub fn accept(&self) -> Option<Arc<Self>> {
        match self.inner.try_get().unwrap() {
            SocketWrapper::TcpServer(tcp_server) => {
                tcp_server.accept().map(|x| {
                    let inner: LazyInit<SocketWrapper<T>> = LazyInit::new();
                    inner.init_by(SocketWrapper::TcpConnection(x));
                    Arc::new(Self {
                        domain: self.domain,
                        net_type: self.net_type,
                        inner,
                    })
                })
            },
            _ => {
                None
            }
        }
    }

    pub fn add_wait_queue(&self, target_ip: u32, target_port: u16) {
        // self.inner.lock().queue.push_back(WaitQueue {
        //     target_port,
        //     target_ip,
        // })
    }

    // pub fn connect(&self, remote: SocketAddrV4) -> Result<
}
