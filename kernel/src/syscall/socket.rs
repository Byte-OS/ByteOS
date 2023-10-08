use core::cmp;
use core::net::{Ipv4Addr, SocketAddrV4};

use alloc::sync::Arc;
use devices::get_net_device;
use executor::{current_user_task, yield_now, AsyncTask, FileItem};
use log::{debug, warn};
use lose_net_stack::connection::NetServer;
use lose_net_stack::net_trait::NetInterface;

use lose_net_stack::results::NetServerError;
use lose_net_stack::MacAddress;
use sync::Lazy;
use vfscore::OpenFlags;

use crate::socket::{self, NetType};
use crate::syscall::fd::sys_pipe2;

use super::consts::{LinuxError, UserRef};

type Socket = socket::Socket;

#[derive(Debug)]
pub struct NetMod;

impl NetInterface for NetMod {
    fn send(data: &[u8]) {
        get_net_device(0)
            .send(data)
            .expect("can't send data in net interface");
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
    let task = current_user_task();
    debug!(
        "[task {}] sys_socket @ domain: {:#x}, net_type: {:#x}, protocol: {:#x}",
        task.get_task_id(),
        domain,
        net_type,
        protocol
    );
    let fd = task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    log::debug!(
        "net_type: {:?}",
        NetType::from_usize(net_type).ok_or(LinuxError::EINVAL)?
    );
    let net_type = NetType::from_usize(net_type).ok_or(LinuxError::EINVAL)?;
    let socket = Socket::new(domain, net_type);
    task.set_fd(fd, FileItem::new_dev(socket));
    Ok(fd)
}

pub async fn sys_socket_pair(
    domain: usize,
    net_type: usize,
    protocol: usize,
    socket_vector: *mut u32,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_socket_pair @ domain: {} net_type: {:#x} protocol: {} socket_vector: {:?}",
        domain, net_type, protocol, socket_vector
    );
    sys_pipe2((socket_vector as usize).into(), 0).await?;
    Ok(0)
}

pub async fn sys_bind(
    socket_fd: usize,
    addr_ptr: UserRef<SocketAddrIn>,
    address_len: usize,
) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] sys_bind @ socket: {:#x}, addr_ptr: {}, address_len: {:#x}",
        task.get_task_id(),
        socket_fd,
        addr_ptr,
        address_len
    );
    let socket_addr = addr_ptr.get_mut();
    debug!("try to bind {:?} to socket {}", socket_addr, socket_fd);
    let socket = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;

    let net_server = NET_SERVER.clone();
    let port = socket_addr.in_port.to_be();
    debug!("read port {}", port);

    if socket_addr.family != 0x02 {
        warn!("only support IPV4 now");
        return Err(LinuxError::EAFNOSUPPORT);
    }

    match socket.net_type {
        NetType::STEAM => {
            if net_server.tcp_is_used(port) {
                let sock = socket.reuse(port);
                task.set_fd(socket_fd, FileItem::new_dev(Arc::new(sock)));
                return Ok(0);
            }
        }
        NetType::DGRAME => {
            if net_server.udp_is_used(port) {
                let sock = socket.reuse(port);
                task.set_fd(socket_fd, FileItem::new_dev(Arc::new(sock)));
                return Ok(0);
            }
        }
        NetType::RAW => {}
    }

    let local = SocketAddrV4::new(socket_addr.addr, port);
    socket
        .inner
        .clone()
        .bind(local)
        .map_err(|_| LinuxError::EALREADY)?;
    debug!("socket_addr: {:#x?}", socket_addr);
    Ok(0)
}

pub async fn sys_listen(socket_fd: usize, backlog: usize) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] sys_listen @ socket_fd: {:#x}, backlog: {:#x}",
        task.get_task_id(),
        socket_fd,
        backlog
    );
    let _ = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?
        .inner
        .clone()
        .listen();
    Ok(0)
}

pub async fn sys_accept(
    socket_fd: usize,
    socket_addr: usize,
    len: usize,
) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] sys_accept @ socket_fd: {:#x}, socket_addr: {:#x}, len: {:#x}",
        task.get_task_id(),
        socket_fd,
        socket_addr,
        len
    );
    let file = task.get_fd(socket_fd).ok_or(LinuxError::EINVAL)?;
    let socket = file
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;
    debug!("flags: {:?}", file.flags.lock());
    let fd = task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    loop {
        if let Ok(new_socket) = socket.inner.accept() {
            task.set_fd(
                fd,
                FileItem::new_dev(Socket::new_with_inner(
                    socket.domain,
                    socket.net_type,
                    new_socket,
                )),
            );
            break;
        }

        if task.tcb.read().signal.has_signal() {
            return Err(LinuxError::EINTR);
        }

        yield_now().await;
    }
    Ok(fd)
}

pub async fn sys_connect(
    socket_fd: usize,
    socket_addr: UserRef<SocketAddrIn>,
    len: usize,
) -> Result<usize, LinuxError> {
    let task = current_user_task();
    warn!(
        "[task {}] sys_connect @ socket_fd: {:#x}, socket_addr: {:#x?}, len: {:#x}",
        task.get_task_id(),
        socket_fd,
        socket_addr,
        len
    );
    let socket = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;

    let socket_addr = socket_addr.get_mut();
    let remote = SocketAddrV4::new(socket_addr.addr, socket_addr.in_port.to_be());
    loop {
        match socket.inner.clone().connect(remote) {
            Err(NetServerError::Blocking) => {}
            _ => break,
        }
        yield_now().await;
    }
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
    let file = task.get_fd(socket_fd).ok_or(LinuxError::EINVAL)?;
    let socket = file
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;

    let (data, remote) = loop {
        let res = socket.recv_from();

        match res {
            Ok(r) => break r,
            Err(_) => {
                if file.flags.lock().contains(OpenFlags::O_NONBLOCK) {
                    return Err(LinuxError::EAGAIN);
                }
                yield_now().await
            }
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

pub async fn sys_getpeername(
    socket_fd: usize,
    addr_ptr: UserRef<SocketAddrIn>,
    len: usize,
) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] sys_getpeername @ socket_fd: {:#x}, addr_ptr: {}, len: {:#x}",
        task.get_task_id(),
        socket_fd,
        addr_ptr,
        len
    );
    let socket = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;
    if addr_ptr.is_valid() {
        let socket_address = socket.inner.get_remote().expect("can't get socket address");
        let socket_addr = addr_ptr.get_mut();
        socket_addr.family = 2;
        socket_addr.addr = *socket_address.ip();
        socket_addr.in_port = socket_address.port().to_be();
        debug!(
            "[task {}] socket address: {:?}",
            task.get_task_id(),
            socket_address
        );
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
    log::warn!("[task {}]sys_setsockopt @ socket: {:#x}, level: {:#x}, optname: {:#x}, optval: {:#x}, optlen: {:#x}", current_user_task().get_task_id(), socket, level, optname, optval, optlen);
    // Ok(0)但在网络游戏这种实时通信中，这种减少包的做法，如果网络较差的时候，可能会引起比较大的波动，比如玩家正在PK，发了技能没有很快的反馈，过一会儿很多技能效果一起回来，这个体验是比较差的。

    // 0x1a SO_ATTACH_FILTER
    // match optname {
    //     0x1 | 0x2 | 0x1a => Ok(0),
    //     _ => {
    //         Err(LinuxError::EPERM)
    //     }
    // }
    Ok(0)
}

pub async fn sys_getsockopt(
    socket: usize,
    level: usize,
    optname: usize,
    optval: *mut u32,
    optlen: *mut u32,
) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!("[task {}] sys_getsockopt @ socket: {:#x}, level: {:#x}, optname: {:#x}, optval: {:#x?}, optlen: {:#x?}", 
    task.get_task_id(), socket, level, optname, optval, optlen);
    unsafe {
        let optval = optval.as_mut().unwrap();
        let _optlen = optlen.as_mut().unwrap();

        match optname {
            // send buffer
            0x7 => *optval = 32000,
            // recv buffer
            0x8 => *optval = 32000,
            0x2 => *optval = 2000,
            _ => {
                // *optval = 2000;
            }
        }
        // debug!("ptr value: {:?}", optval);
    }
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
        task.get_task_id(),
        socket_fd,
        buffer_ptr,
        len,
        flags
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
            .bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 0))
            .map_err(|_| LinuxError::EALREADY)?;
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
    debug!(
        "[task {}] sys_shutdown socket_fd: {:#x}, how: {:#x}",
        task.get_task_id(),
        socket_fd,
        how
    );
    let _ = task
        .get_fd(socket_fd)
        .ok_or(LinuxError::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?
        .inner
        .close();
    Ok(0)
}

pub async fn sys_accept4(
    socket_fd: usize,
    socket_addr: UserRef<SocketAddrIn>,
    len: usize,
    flags: usize,
) -> Result<usize, LinuxError> {
    let task = current_user_task();
    let flags = OpenFlags::from_bits_truncate(flags);
    log::info!(
        "[task {}] sys_accept4 @ socket_fd: {:#x}, socket_addr: {:#x?}, len: {:#x}, flags: {:?}",
        task.get_task_id(),
        socket_fd,
        socket_addr,
        len,
        flags
    );
    let file = task.get_fd(socket_fd).ok_or(LinuxError::EINVAL)?;
    let socket = file
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;
    let fd = task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    loop {
        if let Ok(new_socket) = socket.inner.accept() {
            let sa = socket_addr.get_mut();
            sa.family = 2;
            sa.in_port = new_socket.get_remote().unwrap().port();
            sa.addr = new_socket.get_remote().unwrap().ip().clone();
            let new_file = FileItem::new_dev(Socket::new_with_inner(
                socket.domain,
                socket.net_type,
                new_socket,
            ));
            *new_file.flags.lock() = flags;
            task.set_fd(fd, new_file);
            break Ok(fd);
        } else if file.flags.lock().contains(OpenFlags::O_NONBLOCK) {
            break Err(LinuxError::EAGAIN);
        }
        yield_now().await;
    }
}
