use alloc::{sync::Arc, vec::Vec};
use executor::{current_task, thread, Executor, KernelTask, UserTask};

use crate::syscall::exec_with_process;

use self::{initproc::initproc, user::entry::user_entry};

mod async_ops;
pub mod elf;
mod initproc;
pub mod kernel;
pub mod user;

pub use async_ops::{
    futex_requeue, futex_wake, NextTick, WaitFutex, WaitHandleAbleSignal, WaitPid, WaitSignal,
};

pub enum UserTaskControlFlow {
    Continue,
    Break,
}

#[allow(dead_code)]
pub async fn handle_net() {
    // let lose_stack = LoseStack::new(
    //     IPv4::new(10, 0, 2, 15),
    //     MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
    // );

    // let mut buffer = vec![0u8; 2048];
    // loop {
    //     if TASK_QUEUE.lock().len() == 1 {
    //         break;
    //     }
    //     let rlen = NET_DEVICES.lock()[0].recv(&mut buffer).unwrap_or(0);
    //     if rlen != 0 {
    //         let packet = lose_stack.analysis(&buffer[..rlen]);
    //         debug!("packet: {:?}", packet);
    //         match packet {
    //             Packet::ARP(arp_packet) => {
    //                 debug!("receive arp packet: {:?}", arp_packet);
    //                 let reply_packet = arp_packet
    //                     .reply_packet(lose_stack.ip, lose_stack.mac)
    //                     .expect("can't build reply");
    //                 NET_DEVICES.lock()[0]
    //                     .send(&reply_packet.build_data())
    //                     .expect("can't send net data");
    //             }
    //             Packet::UDP(udp_packet) => {
    //                 debug!("udp_packet: {:?}", udp_packet);
    //             }
    //             Packet::TCP(tcp_packet) => {
    //                 let net = NET_DEVICES.lock()[0].clone();
    //                 if tcp_packet.flags == TcpFlags::S {
    //                     // receive a tcp connect packet
    //                     let mut reply_packet = tcp_packet.ack();
    //                     reply_packet.flags = TcpFlags::S | TcpFlags::A;
    //                     if let Some(socket) = PORT_TABLE.lock().get(&tcp_packet.dest_port) {
    //                         // TODO: create a new socket as the child of this socket.
    //                         // and this is receive a child.
    //                         // TODO: specific whether it is tcp or udp

    //                         info!(
    //                             "[TCP CONNECT]{}:{}(MAC:{}) -> {}:{}(MAC:{})  len:{}",
    //                             tcp_packet.source_ip,
    //                             tcp_packet.source_port,
    //                             tcp_packet.source_mac,
    //                             tcp_packet.dest_ip,
    //                             tcp_packet.dest_port,
    //                             tcp_packet.dest_mac,
    //                             tcp_packet.data_len
    //                         );
    //                         if socket.net_type == NetType::STEAM {
    //                             socket.add_wait_queue(
    //                                 tcp_packet.source_ip.to_u32(),
    //                                 tcp_packet.source_port,
    //                             );
    //                             let reply_data = &reply_packet.build_data();
    //                             net.send(&reply_data).expect("can't send to net");
    //                         }
    //                     }
    //                 } else if tcp_packet.flags.contains(TcpFlags::F) {
    //                     // tcp disconnected
    //                     info!(
    //                         "[TCP DISCONNECTED]{}:{}(MAC:{}) -> {}:{}(MAC:{})  len:{}",
    //                         tcp_packet.source_ip,
    //                         tcp_packet.source_port,
    //                         tcp_packet.source_mac,
    //                         tcp_packet.dest_ip,
    //                         tcp_packet.dest_port,
    //                         tcp_packet.dest_mac,
    //                         tcp_packet.data_len
    //                     );
    //                     let reply_packet = tcp_packet.ack();
    //                     net.send(&reply_packet.build_data())
    //                         .expect("can't send to net");

    //                     let mut end_packet = reply_packet.ack();
    //                     end_packet.flags |= TcpFlags::F;
    //                     net.send(&end_packet.build_data())
    //                         .expect("can't send to net");
    //                 } else {
    //                     info!(
    //                         "{}:{}(MAC:{}) -> {}:{}(MAC:{})  len:{}",
    //                         tcp_packet.source_ip,
    //                         tcp_packet.source_port,
    //                         tcp_packet.source_mac,
    //                         tcp_packet.dest_ip,
    //                         tcp_packet.dest_port,
    //                         tcp_packet.dest_mac,
    //                         tcp_packet.data_len
    //                     );

    //                     if tcp_packet.flags.contains(TcpFlags::A) && tcp_packet.data_len == 0 {
    //                         continue;
    //                     }

    //                     if let Some(socket) = PORT_TABLE.lock().get(&tcp_packet.dest_port) {
    //                         let socket_inner = socket.inner.lock();
    //                         let client = socket_inner.clients.iter().find(|x| match x.upgrade() {
    //                             Some(x) => {
    //                                 let client_inner = x.inner.lock();
    //                                 client_inner.target_ip == tcp_packet.source_ip.to_u32()
    //                                     && client_inner.target_port == tcp_packet.source_port
    //                             }
    //                             None => false,
    //                         });

    //                         client.map(|x| {
    //                             let socket = x.upgrade().unwrap();
    //                             let mut socket_inner = socket.inner.lock();

    //                             socket_inner.datas.push_back(tcp_packet.data.to_vec());
    //                             let reply = tcp_packet.reply(&[0u8; 0]);
    //                             socket_inner.ack = reply.ack;
    //                             socket_inner.seq = reply.seq;
    //                             socket_inner.flags = reply.flags.bits();
    //                         });
    //                     }

    //                     // handle tcp data
    //                     // receive_tcp(&mut net, &tcp_packet)
    //                 }
    //             }
    //             Packet::ICMP() => {
    //                 debug!("receive ICMP packet")
    //             }
    //             Packet::IGMP() => {
    //                 debug!("receive IGMP packet")
    //             }
    //             Packet::Todo(_) => {
    //                 debug!("receive IGMP packet")
    //             }
    //             Packet::None => {
    //                 debug!("receive IGMP packet")
    //             }
    //         }
    //     }
    //     yield_now().await;
    // }
}

pub fn init() {
    let mut exec = Executor::new();
    exec.spawn(KernelTask::new(initproc()));
    #[cfg(feature = "net")]
    exec.spawn(KernelTask::new(handle_net()));
    // exec.spawn()
    exec.run();
}

pub async fn add_user_task(filename: &str, args: Vec<&str>, _envp: Vec<&str>) {
    let task = UserTask::new(user_entry(), Arc::downgrade(&current_task()));
    exec_with_process(task.clone(), filename, args)
        .await
        .expect("can't add task to excutor");
    thread::spawn(task.clone());
}
