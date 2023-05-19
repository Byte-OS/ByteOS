use core::marker::PhantomData;

use alloc::{
    collections::VecDeque,
    sync::{Arc, Weak},
    vec::Vec,
};
use log::debug;
use sync::Mutex;
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
            _ => None,
        }
    }
}

pub struct WaitQueue {
    target_port: u16,
    target_ip: u32,
}

pub struct SocketInner<T: SocketOps> {
    pub listened: bool,
    pub clients: Vec<Weak<Socket<T>>>,
    pub port: u16,
    pub target_ip: u32,
    pub target_port: u16,
    pub datas: Vec<Vec<u8>>,
    pub queue: VecDeque<WaitQueue>,
}

pub trait SocketOps: Sync + Send {
    fn tcp_send(&self, data: &[u8]);
}

#[allow(dead_code)]
pub struct Socket<T: SocketOps> {
    pub domain: usize,
    pub net_type: NetType,
    pub inner: Mutex<SocketInner<T>>,
    pub ops: PhantomData<T>,
}

impl<T: SocketOps + 'static> INodeInterface for Socket<T> {
    fn metadata(&self) -> VfsResult<Metadata> {
        Ok(Metadata {
            filename: "",
            inode: 0,
            file_type: vfscore::FileType::Socket,
            size: 0,
            childrens: 0,
        })
    }
}

impl<T: SocketOps> Socket<T> {
    pub fn new(domain: usize, net_type: NetType) -> Arc<Self> {
        Arc::new(Self {
            domain,
            net_type,
            inner: Mutex::new(SocketInner {
                listened: false,
                clients: vec![],
                port: 0,
                target_ip: 0,
                target_port: 0,
                datas: Vec::new(),
                queue: VecDeque::new(),
            }),
            ops: PhantomData,
        })
    }

    pub fn bind(&self, port: u16) {
        self.inner.lock().port = port;
    }

    pub fn listen(&self) {
        self.inner.lock().listened = true;
    }

    pub fn add_socket(&self, child: Arc<Socket<T>>) {
        let mut inner = self.inner.lock();
        inner.clients.drain_filter(|x| x.upgrade().is_none());
        inner.clients.push(Arc::downgrade(&child));
    }

    pub fn conn_num(&self) -> usize {
        let mut inner = self.inner.lock();
        inner.clients.drain_filter(|x| x.upgrade().is_none());
        inner.clients.len()
    }

    pub fn accept(&self) -> Option<Arc<Self>> {
        let que_top = self.inner.lock().queue.pop_front();
        if let Some(conn) = que_top {
            let inner = self.inner.lock();
            let new_socket = Arc::new(Self {
                domain: self.domain,
                net_type: self.net_type,
                inner: Mutex::new(SocketInner {
                    listened: inner.listened,
                    clients: Vec::new(),
                    port: inner.port,
                    target_ip: conn.target_ip,
                    target_port: conn.target_port,
                    datas: Vec::new(),
                    queue: VecDeque::new(),
                }),
                ops: self.ops,
            });
            drop(inner);
            self.add_socket(new_socket.clone());
            Some(new_socket)
        } else {
            None
        }
    }

    pub fn add_wait_queue(&self, target_ip: u32, target_port: u16) {
        self.inner.lock().queue.push_back(WaitQueue {
            target_port,
            target_ip,
        })
    }
}
