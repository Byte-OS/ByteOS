use core::cmp;
use core::net::{Ipv4Addr, SocketAddrV4};

use alloc::sync::Arc;
use executor::{current_user_task, yield_now, FileItem, UserTask};
use fs::socket::{NetType, self, SocketWrapper};
use fs::INodeInterface;
use log::debug;
use lose_net_stack::connection::{NetServer, tcp};
use lose_net_stack::net_trait::NetInterface;


use lose_net_stack::MacAddress;
use sync::{Lazy, LazyInit};

use super::consts::{LinuxError, UserRef};

type Socket = socket::Socket<NetMod>;

#[derive(Debug)]
pub struct NetMod;

impl NetInterface for NetMod {
    fn send(data: &[u8]) {
        debug!("do nothing");
        // NET.lock().as_mut().unwrap().send(data);
    }

    fn local_mac_address() -> MacAddress {
        MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56])
    }
}

pub static NET_SERVER: Lazy<Arc<NetServer<NetMod>>> = Lazy::new(|| Arc::new(NetServer::new(
    MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
    Ipv4Addr::new(10, 0, 2, 15)
)));

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

    if socket_addr.in_port == 0 {
        socket_addr.in_port = match socket.net_type {
            NetType::STEAM => {
                net_server.alloc_tcp_port()
            },
            NetType::DGRAME => {
                net_server.alloc_udp_port()
            },
            NetType::RAW => 0,
        }.to_be();
    }

    let port = socket_addr.in_port.to_be();

    match socket.net_type {
        NetType::STEAM => {
            if net_server.tcp_is_used(port) {
                return Err(LinuxError::EBUSY);
            }
        },
        NetType::DGRAME => {
            if net_server.udp_is_used(port) {
                return Err(LinuxError::EBUSY);
            }
        },
        NetType::RAW => {},
    }

    if port != 0 {
        // port_bind(port, socket)
        match socket.net_type {
            NetType::STEAM => {
                let server = net_server.listen_tcp(port).expect("can't listen to UDP");
                socket.inner.init_by(SocketWrapper::TcpServer(server));
            },
            NetType::DGRAME => {
                let server = net_server.listen_udp(port).expect("can't listen to UDP");
                socket.inner.init_by(SocketWrapper::Udp(server));
            },
            NetType::RAW => {},
        }
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
        .get_bare_file()
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
    let fd = task.alloc_fd().ok_or(LinuxError::EMFILE)?;

    if !socket.inner.is_init() {
        let port = NET_SERVER.alloc_tcp_port();
        socket.inner.init_by(SocketWrapper::TcpServer(NET_SERVER.listen_tcp(port).expect("can't create socket udp @ send")));
    }
    let socket_addr = socket_addr.get_mut();
    let remote = SocketAddrV4::new(socket_addr.addr, socket_addr.in_port.to_be());
    match socket.inner.try_get().unwrap() {
        SocketWrapper::TcpServer(tcp_serve) => {
            let tcp_conn = tcp_serve.connect(remote).ok_or(LinuxError::EMFILE)?;
            let inner: LazyInit<SocketWrapper<NetMod>> = LazyInit::new();
            inner.init_by(SocketWrapper::TcpConnection(tcp_conn));
            let new_socket = Socket::new(socket.domain, socket.net_type);
            task.set_fd(fd, Some(FileItem::new(new_socket, Default::default())));
        },
        _ => {}
    }
    // Ok(fd)
    Ok(0)
}

pub async fn sys_recvfrom(
    socket_fd: usize,
    buffer_ptr: UserRef<u8>,
    len: usize,
    flags: usize,
    addr: usize,
    addr_len: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_recvfrom @ socket_fd: {:#x}, buffer_ptr: {}, len: {:#x}, flags: {:#x}, addr: {:#x}, addr_len: {:#x}", 
        socket_fd, buffer_ptr, len, flags, addr, addr_len
    );
    let buffer = buffer_ptr.slice_mut_with_len(len);
    let task = current_user_task();
    let socket = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;
    match socket.inner.try_get().unwrap() {
        SocketWrapper::Udp(udp_client) => {
            let rlen = loop {
                if let Some(buf) = udp_client.receve_from() {
                    let rlen = cmp::min(buf.data.len(), buffer.len());
                    buffer[..rlen].copy_from_slice(&buf.data[..rlen]);
                    break rlen;
                }
                yield_now().await;
            };
            Ok(rlen)
        },
        _ => {
            Err(LinuxError::EPERM)
        }
    }
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

pub async fn sys_sendto(
    socket_fd: usize,
    buffer_ptr: UserRef<u8>,
    len: usize,
    flags: usize,
    addr_ptr: UserRef<SocketAddrIn>,
    _address_len: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_send @ socket_fd: {:#x}, buffer_ptr: {}, len: {:#x}, flags: {:#x}",
        socket_fd, buffer_ptr, len, flags
    );
    let buffer = buffer_ptr.slice_mut_with_len(len);
    let task = current_user_task();
    let socket = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;

    if !socket.inner.is_init() {
        let port = NET_SERVER.alloc_udp_port();
        socket.inner.init_by(SocketWrapper::Udp(NET_SERVER.listen_udp(port).expect("can't create socket udp @ send")));
    }
    match socket.inner.try_get().unwrap() {
        SocketWrapper::Udp(udp_client) => {
            let socket_addr = addr_ptr.get_mut();
            udp_client.sendto(SocketAddrV4::new(socket_addr.addr, socket_addr.in_port.to_be()), &buffer);
            // SocketOpera::udp_send(
            //     socket_addr.addr.to_be(),
            //     socket_addr.in_port.to_be(),
            //     &buffer,
            // );
            Ok(buffer.len())
        },
        _ => {
            Err(LinuxError::EPERM)
        }
    }
}

pub async fn accept(fd: usize, task: Arc<UserTask>, socket: Arc<Socket>) {
    loop {
        if let Some(new_socket) = socket.accept() {
            task.set_fd(fd, Some(FileItem::new(new_socket, Default::default())));
            return;
        }
        yield_now().await;
    }
}
