use core::cmp;
use core::net::{Ipv4Addr, SocketAddrV4};

use alloc::sync::Arc;
use executor::{current_user_task, yield_now, FileItem, UserTask, AsyncTask};
use log::debug;
use lose_net_stack::connection::NetServer;
use lose_net_stack::net_trait::NetInterface;

use lose_net_stack::MacAddress;
use sync::Lazy;

use crate::socket::{self, NetType};

use super::consts::{LinuxError, UserRef};

type Socket = socket::Socket;

#[derive(Debug)]
pub struct NetMod;

impl NetInterface for NetMod {
    fn send(_data: &[u8]) {
        debug!("do nothing");
        // NET.lock().as_mut().unwrap().send(data);
    }

    fn local_mac_address() -> MacAddress {
        MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56])
    }
}

pub static NET_SERVER: Lazy<Arc<NetServer<NetMod>>> = Lazy::new(|| {
    Arc::new(NetServer::new(
        MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
        Ipv4Addr::new(10, 0, 2, 15),
    ))
});

#[derive(Debug)]
#[allow(dead_code)]
struct SocketAddr {
    sa_family: u16,
    sa_data: [u8; 14],
}

#[derive(Debug)]
#[repr(C)]
pub struct SocketAddrIn {
    family: u16,
    in_port: u16,
    addr: Ipv4Addr,
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
    let net_type = NetType::from_usize(net_type).ok_or(LinuxError::EINVAL)?;

    let socket = Socket::new(domain, net_type);
    task.set_fd(fd, Some(FileItem::new(socket, Default::default())));
    Ok(fd)
}

pub async fn sys_bind(
    socket_fd: usize,
    addr_ptr: UserRef<SocketAddrIn>,
    address_len: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_bind @ socket: {:#x}, addr_ptr: {}, address_len: {:#x}",
        socket_fd, addr_ptr, address_len
    );
    let task = current_user_task();
    let socket_addr = addr_ptr.get_mut();

    let socket = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;

    let net_server = NET_SERVER.clone();
    let port = socket_addr.in_port.to_be();

    match socket.net_type {
        NetType::STEAM => {
            if net_server.tcp_is_used(port) {
                return Err(LinuxError::EBUSY);
            }
        }
        NetType::DGRAME => {
            if net_server.udp_is_used(port) {
                return Err(LinuxError::EBUSY);
            }
        }
        NetType::RAW => {}
    }

    // if port != 0 {
    //     // port_bind(port, socket)
    //     socket.inner.clone().bind(SocketAddrV4::new(socket_addr.addr, port));
    // }
    let local = SocketAddrV4::new(socket_addr.addr, port);
    socket.inner.clone().bind(local);
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
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?
        .inner
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
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;
    let fd = task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    accept(fd, task, socket).await;
    Ok(fd)
}

pub async fn sys_connect(
    socket_fd: usize,
    socket_addr: UserRef<SocketAddrIn>,
    len: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_connect @ socket_fd: {:#x}, socket_addr: {:#x?}, len: {:#x}",
        socket_fd, socket_addr, len
    );
    let task = current_user_task();
    let socket = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;

    let socket_addr = socket_addr.get_mut();
    let remote = SocketAddrV4::new(socket_addr.addr, socket_addr.in_port.to_be());
    socket.inner.clone().connect(remote);
    yield_now().await;
    Ok(0)
}

pub async fn sys_recvfrom(
    socket_fd: usize,
    buffer_ptr: UserRef<u8>,
    len: usize,
    flags: usize,
    addr: UserRef<SocketAddrIn>,
    addr_len: UserRef<usize>,
) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] sys_recvfrom @ socket_fd: {:#x}, buffer_ptr: {}, len: {:#x}, flags: {:#x}, addr: {:#x?}, addr_len: {:#x?}", 
        task.get_task_id(), socket_fd, buffer_ptr, len, flags, addr, addr_len
    );
    let buffer = buffer_ptr.slice_mut_with_len(len);
    let socket = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;

        
    let (data, remote) = loop {
        let res = socket.recv_from();

        match res {
            Ok(r) => break r,
            Err(_) => yield_now().await,
        }
    };
    let rlen = cmp::min(data.len(), buffer.len());
    buffer[..rlen].copy_from_slice(&data[..rlen]);

    if addr.is_valid() {
        let socket_addr = addr.get_mut();
        socket_addr.in_port = remote.port().to_be();
        socket_addr.family = 2;
        socket_addr.addr = *remote.ip();
    }
    Ok(rlen)
}

pub async fn sys_getsockname(
    socket_fd: usize,
    addr_ptr: UserRef<SocketAddrIn>,
    len: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_getsockname @ socket_fd: {:#x}, addr_ptr: {}, len: {:#x}",
        socket_fd, addr_ptr, len
    );
    let task = current_user_task();
    let socket = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;
    if addr_ptr.is_valid() {
        let socket_address = socket.inner.get_local().expect("can't get socket address");
        let socket_addr = addr_ptr.get_mut();
        socket_addr.family = 2;
        socket_addr.addr = *socket_address.ip();
        socket_addr.in_port = socket_address.port().to_be();
        debug!("socket address: {:?}", socket_address);
    }
    Ok(0)
}

pub async fn sys_setsockopt(
    socket: usize,
    level: usize,
    optname: usize,
    optval: usize,
    optlen: usize,
) -> Result<usize, LinuxError> {
    debug!("sys_setsockopt @ socket: {:#x}, level: {:#x}, optname: {:#x}, optval: {:#x}, optlen: {:#x}", socket, level, optname, optval, optlen);
    Ok(0)
}

pub async fn sys_getsockopt(
    socket: usize,
    level: usize,
    optname: usize,
    optval: usize,
    optlen: usize,
) -> Result<usize, LinuxError> {
    debug!("sys_getsockopt @ socket: {:#x}, level: {:#x}, optname: {:#x}, optval: {:#x}, optlen: {:#x}", socket, level, optname, optval, optlen);
    Ok(0)
}

pub async fn sys_sendto(
    socket_fd: usize,
    buffer_ptr: UserRef<u8>,
    len: usize,
    flags: usize,
    addr_ptr: UserRef<SocketAddrIn>,
    _address_len: usize,
) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] sys_send @ socket_fd: {:#x}, buffer_ptr: {}, len: {:#x}, flags: {:#x}",
        task.get_task_id(), socket_fd, buffer_ptr, len, flags
    );
    let buffer = buffer_ptr.slice_mut_with_len(len);
    let socket = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;

    debug!("send");

    if socket.inner.get_local().unwrap().port() == 0 {
        socket
            .inner
            .clone()
            .bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 0));
    }

    let remote = if addr_ptr.is_valid() {
        let socket_addr = addr_ptr.get_mut();
        Some(SocketAddrV4::new(
            socket_addr.addr,
            socket_addr.in_port.to_be(),
        ))
    } else {
        None
    };

    let wlen = socket.inner.sendto(buffer, remote).expect("buffer");
    Ok(wlen)
}

pub async fn sys_shutdown(socket_fd: usize, how: usize) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!("[task {}] sys_shutdown socket_fd: {:#x}, how: {:#x}", task.get_task_id(), socket_fd, how);
    Ok(0)
}


pub async fn accept(fd: usize, task: Arc<UserTask>, socket: Arc<Socket>) {
    loop {
        if let Ok(new_socket) = socket.inner.accept() {
            task.set_fd(
                fd,
                Some(FileItem::new(
                    Socket::new_with_inner(socket.domain, socket.net_type, new_socket),
                    Default::default(),
                )),
            );
            return;
        }
        yield_now().await;
    }
}
