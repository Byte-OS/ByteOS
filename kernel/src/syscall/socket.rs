use alloc::{collections::BTreeMap, sync::Arc};
use devices::NET_DEVICES;
use executor::{current_user_task, yield_now, UserTask};
use fs::socket::{self, NetType, SocketOps};
use fs::INodeInterface;
use log::debug;
use lose_net_stack::packets::tcp::TCPPacket;
use lose_net_stack::{IPv4, MacAddress, TcpFlags};
use sync::Mutex;

use super::consts::{LinuxError, UserRef};

type Socket = socket::Socket<SocketOpera>;

pub static PORT_TABLE: Mutex<BTreeMap<u16, Arc<Socket>>> = Mutex::new(BTreeMap::new());

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
    addr_ptr: UserRef<SocketAddrIn>,
    address_len: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_bind @ socket: {:#x}, addr_ptr: {}, address_len: {:#x}",
        socket_fd, addr_ptr, address_len
    );
    let task = current_user_task();
    let socket_addr = addr_ptr.get_mut();
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
    Ok(fd)
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
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;
    let rlen = loop {
        let rlen = socket.read(buffer).expect("cant recv from socket");
        if rlen != 0 {
            break rlen;
        }
        yield_now().await;
    };
    Ok(rlen)
}

pub async fn sys_sendto(
    socket_fd: usize,
    buffer_ptr: UserRef<u8>,
    len: usize,
    flags: usize,
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
        .downcast_arc::<Socket>()
        .map_err(|_| LinuxError::EINVAL)?;
    let wlen = socket.write(buffer).expect("can't send to socket");
    Ok(wlen)
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
    // fn tcp_send(ip: u32, port: u16, ack: usize, data: &[u8]) {
    //     debug!("tcp send to");
    //     todo!()
    // }
    fn tcp_send(
        ip: u32,
        port: u16,
        ack: u32,
        seq: u32,
        flags: u8,
        win: u16,
        urg: u16,
        data: &[u8],
    ) {
        // let lose_stack = LoseStack::new(
        //     IPv4::new(10, 0, 2, 15),
        //     MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
        // );

        let tcp_packet = TCPPacket {
            source_ip: IPv4::new(10, 0, 2, 15),
            source_mac: MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
            source_port: 2000,
            dest_ip: IPv4::from_u32(ip),
            dest_mac: MacAddress::new([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
            dest_port: port,
            data_len: data.len(),
            seq,
            ack,
            flags: TcpFlags::from_bits_truncate(flags),
            win,
            urg,
            data,
        };
        NET_DEVICES.lock()[0]
            .send(&tcp_packet.build_data())
            .expect("can't send date to net device");
        // debug!("tcp send to");
        // todo!()
    }
}
