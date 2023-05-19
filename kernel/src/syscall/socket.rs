use alloc::{collections::BTreeMap, sync::Arc};
use executor::{current_user_task, yield_now, UserTask};
use fs::socket::{self, NetType, SocketOps};
use log::debug;
use sync::Mutex;

use crate::syscall::c2rust_ref;

use super::consts::LinuxError;

type Socket = socket::Socket<SocketOpera>;

pub static PORT_TABLE: Mutex<BTreeMap<u16, Arc<Socket>>> = Mutex::new(BTreeMap::new());

#[derive(Debug)]
struct SocketAddr {
    sa_family: u16,
    sa_data: [u8; 14],
}

#[derive(Debug)]
#[repr(C)]
pub struct SocketAddrIn {
    family: u16,
    in_port: u16,
    addr: u32,
    sin_zero: [u8; 8],
}

pub async fn sys_socket(
    domain: usize,
    net_type: usize,
    protocol: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_socket @ domain: {:#x}, net_type: {:#x}, protocol: {:#x}",
        domain, net_type, protocol
    );
    let task = current_user_task();
    let fd = task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    debug!(
        "net_type: {:?}",
        NetType::from_usize(net_type).ok_or(LinuxError::EINVAL)?
    );
    let socket = Socket::new(
        domain,
        NetType::from_usize(net_type).ok_or(LinuxError::EINVAL)?,
    );
    task.set_fd(fd, Some(socket));
    Ok(fd)
}

pub async fn sys_bind(
    socket_fd: usize,
    addr_ptr: usize,
    address_len: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_bind @ socket: {:#x}, addr_ptr: {:#x}, address_len: {:#x}",
        socket_fd, addr_ptr, address_len
    );
    let task = current_user_task();
    let socket_addr = c2rust_ref(addr_ptr as *mut SocketAddrIn);
    let port = socket_addr.in_port.to_be();
    let socket = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;
    if port_used(port) {
        return Err(LinuxError::EBUSY);
    }
    if port != 0 {
        port_bind(port, socket)
    }
    debug!("socket_addr: {:#x?}", socket_addr);
    Ok(0)
}

pub async fn sys_listen(socket_fd: usize, backlog: usize) -> Result<usize, LinuxError> {
    debug!(
        "sys_listen @ socket_fd: {:#x}, backlog: {:#x}",
        socket_fd, backlog
    );
    let task = current_user_task();
    task.get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?
        .listen();
    Ok(0)
}

pub async fn sys_accept(
    socket_fd: usize,
    socket_addr: usize,
    len: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_accept @ socket_fd: {:#x}, socket_addr: {:#x}, len: {:#x}",
        socket_fd, socket_addr, len
    );
    let task = current_user_task();
    let socket = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;
    let fd = task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    accept(fd, task, socket).await;
    Ok(0)
}

pub fn port_used(port: u16) -> bool {
    PORT_TABLE.lock().contains_key(&port)
}

pub fn port_bind(port: u16, socket: Arc<Socket>) {
    socket.bind(port);
    PORT_TABLE.lock().insert(port, socket);
}

pub async fn accept(fd: usize, task: Arc<UserTask>, socket: Arc<Socket>) {
    loop {
        if let Some(new_socket) = socket.accept() {
            task.set_fd(fd, Some(new_socket));
            return;
        }
        yield_now().await;
    }
}

pub struct SocketOpera;

impl SocketOps for SocketOpera {
    fn tcp_send(&self, data: &[u8]) {
        todo!()
    }
}
