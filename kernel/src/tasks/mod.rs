use core::{future::Future, mem::size_of};

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use arch::{get_time, trap_pre_handle, user_restore, Context, ContextOps, VirtPage};
use devices::NET_DEVICES;
use executor::{
    current_task, current_user_task, thread, yield_now, Executor, KernelTask, MemType, UserTask,
};
use fs::socket::NetType;
use log::debug;
use lose_net_stack::{results::Packet, IPv4, LoseStack, MacAddress, TcpFlags};
use signal::SignalFlags;

use crate::syscall::{
    c2rust_ref, consts::SignalUserContext, exec_with_process, syscall, PORT_TABLE,
};

use self::initproc::initproc;

mod async_ops;
pub mod elf;
mod initproc;

pub use async_ops::{futex_requeue, futex_wake, NextTick, WaitFutex, WaitPid, WaitSignal};

#[no_mangle]
// for avoiding the rust cycle check. user extern and nomangle
pub fn user_entry() -> Box<dyn Future<Output = ()> + Send + Sync> {
    Box::new(async { user_entry_inner().await })
}

enum UserTaskControlFlow {
    Continue,
    Break,
}

async fn handle_syscall(task: Arc<UserTask>, cx_ref: &mut Context) -> UserTaskControlFlow {
    let ustart = 0;
    unsafe {
        user_restore(cx_ref);
    }
    task.inner_map(|inner| inner.tms.utime += (get_time() - ustart) as u64);

    let sstart = 0;
    let trap_type = trap_pre_handle(cx_ref);
    match trap_type {
        arch::TrapType::Breakpoint => {}
        arch::TrapType::UserEnvCall => {
            debug!("user env call: {}", cx_ref.syscall_number());
            // if syscall ok
            let args = cx_ref.args();
            let args = [
                args[0], args[1], args[2], args[3], args[4], args[5], args[6],
            ];
            let call_number = cx_ref.syscall_number();
            cx_ref.syscall_ok();
            let result = syscall(call_number, args)
                .await
                .map_or_else(|e| -e.code(), |x| x as isize) as usize;
            debug!("syscall result: {:#X?}", result);
            cx_ref.set_ret(result);
            if result == (-500 as isize) as usize {
                return UserTaskControlFlow::Break;
            }
        }
        arch::TrapType::Time => {
            debug!("time interrupt from user");
        }
        arch::TrapType::Unknown => {
            debug!("unknown trap: {:#x?}", cx_ref);
            panic!("");
        }
        arch::TrapType::StorePageFault(addr) => {
            let vpn = VirtPage::from_addr(addr);
            debug!("store page fault @ {:#x}", addr);
            let mem_tracker = task
                .inner
                .lock()
                .memset
                .iter()
                .find(|x| {
                    x.vpn == vpn
                        && match x.mem_type {
                            MemType::Clone => true,
                            _ => false,
                        }
                })
                .map(|x| x.tracker.clone());

            match mem_tracker {
                Some(tracker) => {
                    let src_ppn = tracker.0;
                    let dst_ppn = task.frame_alloc(vpn, MemType::CodeSection);
                    dst_ppn.copy_value_from_another(src_ppn);
                }
                None => {
                    if (0x7fff0000..0x7ffff000).contains(&addr) {
                        task.frame_alloc(vpn, MemType::Stack);
                    } else {
                        debug!("context: {:#X?}", cx_ref);
                        return UserTaskControlFlow::Break;
                    }
                }
            }
        }
    }
    task.inner_map(|inner| inner.tms.stime += (get_time() - sstart) as u64);
    UserTaskControlFlow::Continue
}

pub async fn handle_signal(task: Arc<UserTask>, signal: SignalFlags) {
    let sigaction = task
        .inner_map(|inner| inner.sigaction.lock().get(signal.num()).unwrap().clone())
        .clone();

    if sigaction.handler == 0 {
        match signal {
            SignalFlags::SIGCANCEL => {
                current_user_task().exit_with_signal(signal.num());
            }
            _ => {}
        }
        return;
    }

    // debug!("sigactions: {:#X?}", sigaction);

    let cx_ref = unsafe { task.get_cx_ptr().as_mut().unwrap() };

    // let store_cx = cx_ref.clone();

    let mut sp = cx_ref.sp();

    sp -= 128;
    sp -= size_of::<SignalUserContext>();
    sp = sp / 16 * 16;

    let cx = c2rust_ref(sp as *mut SignalUserContext);
    let store_cx = cx_ref.clone();
    task.inner_map(|inner| {
        // cx.context.clone_from(&inner.cx);
        cx.pc = inner.cx.sepc();
        cx.sig_mask = sigaction.mask;
        // debug!("pc: {:#X}, mask: {:#X?}", cx.pc, cx.sig_mask);
        inner.cx.set_sepc(sigaction.handler);
        inner.cx.set_ra(sigaction.restorer);
        inner.cx.set_arg0(signal.num());
        inner.cx.set_arg1(0);
        inner.cx.set_arg2(sp);
    });

    loop {
        if let Some(exit_code) = task.exit_code() {
            debug!("program exit with code: {}", exit_code);
            break;
        }

        if let UserTaskControlFlow::Break = handle_syscall(task.clone(), cx_ref).await {
            break;
        }
    }

    // debug!("new pc: {:#X}", cx.pc);
    // store_cx.set_ret(cx_ref.args()[0]);
    cx_ref.clone_from(&store_cx);
    // copy pc from new_pc
    cx_ref.set_sepc(cx.pc);
}

pub async fn user_entry_inner() {
    let mut times = 0;
    loop {
        let task = current_user_task();
        debug!("user_entry, task: {}", task.task_id);
        loop {
            if let Some(signal) = task.inner_map(|x| x.signal.handle_signal()) {
                // debug!("handle signal: {:?}  num: {}", signal, signal.num());
                handle_signal(task.clone(), signal.clone()).await;
            } else {
                break;
            }
        }
        let cx_ref = unsafe { task.get_cx_ptr().as_mut().unwrap() };

        if let Some(exit_code) = task.exit_code() {
            debug!("program exit with code: {}", exit_code);
            break;
        }

        if let UserTaskControlFlow::Break = handle_syscall(task, cx_ref).await {
            break;
        }

        times += 1;

        if times >= 50 {
            times = 0;
            yield_now().await;
        }

        // yield_now().await;
    }
}

pub async fn handle_net() {
    let lose_stack = LoseStack::new(
        IPv4::new(10, 0, 2, 15),
        MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
    );

    let mut buffer = vec![0u8; 2048];
    loop {
        let rlen = NET_DEVICES.lock()[0].recv(&mut buffer).unwrap_or(0);
        if rlen != 0 {
            let packet = lose_stack.analysis(&buffer[..rlen]);
            match packet {
                Packet::ARP(arp_packet) => {
                    debug!("receive arp packet: {:?}", arp_packet);
                    let reply_packet = arp_packet
                        .reply_packet(lose_stack.ip, lose_stack.mac)
                        .expect("can't build reply");
                    NET_DEVICES.lock()[0]
                        .send(&reply_packet.build_data())
                        .expect("can't send net data");
                }
                Packet::UDP(_) => todo!(),
                Packet::TCP(tcp_packet) => {
                    let net = NET_DEVICES.lock()[0].clone();
                    if tcp_packet.flags == TcpFlags::S {
                        // receive a tcp connect packet
                        let mut reply_packet = tcp_packet.ack();
                        reply_packet.flags = TcpFlags::S | TcpFlags::A;
                        if let Some(socket) = PORT_TABLE.lock().get(&tcp_packet.dest_port) {
                            // TODO: create a new socket as the child of this socket.
                            // and this is receive a child.
                            // TODO: specific whether it is tcp or udp

                            info!(
                                "[TCP CONNECT]{}:{}(MAC:{}) -> {}:{}(MAC:{})  len:{}",
                                tcp_packet.source_ip,
                                tcp_packet.source_port,
                                tcp_packet.source_mac,
                                tcp_packet.dest_ip,
                                tcp_packet.dest_port,
                                tcp_packet.dest_mac,
                                tcp_packet.data_len
                            );
                            if socket.net_type == NetType::STEAM {
                                socket.add_wait_queue(
                                    tcp_packet.source_ip.to_u32(),
                                    tcp_packet.source_port,
                                );
                                let reply_data = &reply_packet.build_data();
                                net.send(&reply_data).expect("can't send to net");
                            }
                        }
                    } else if tcp_packet.flags.contains(TcpFlags::F) {
                        // tcp disconnected
                        info!(
                            "[TCP DISCONNECTED]{}:{}(MAC:{}) -> {}:{}(MAC:{})  len:{}",
                            tcp_packet.source_ip,
                            tcp_packet.source_port,
                            tcp_packet.source_mac,
                            tcp_packet.dest_ip,
                            tcp_packet.dest_port,
                            tcp_packet.dest_mac,
                            tcp_packet.data_len
                        );
                        let reply_packet = tcp_packet.ack();
                        net.send(&reply_packet.build_data())
                            .expect("can't send to net");

                        let mut end_packet = reply_packet.ack();
                        end_packet.flags |= TcpFlags::F;
                        net.send(&end_packet.build_data())
                            .expect("can't send to net");
                    } else {
                        info!(
                            "{}:{}(MAC:{}) -> {}:{}(MAC:{})  len:{}",
                            tcp_packet.source_ip,
                            tcp_packet.source_port,
                            tcp_packet.source_mac,
                            tcp_packet.dest_ip,
                            tcp_packet.dest_port,
                            tcp_packet.dest_mac,
                            tcp_packet.data_len
                        );

                        hexdump(tcp_packet.data.as_ref());
                        if tcp_packet.flags.contains(TcpFlags::A) && tcp_packet.data_len == 0 {
                            continue;
                        }

                        // handle tcp data
                        // receive_tcp(&mut net, &tcp_packet)
                    }
                }
                Packet::ICMP() => todo!(),
                Packet::IGMP() => todo!(),
                Packet::Todo(_) => todo!(),
                Packet::None => todo!(),
            }
        }
        yield_now().await;
    }
}

#[no_mangle]
pub fn hexdump(data: &[u8]) {
    const PRELAND_WIDTH: usize = 70;
    println!("{:-^1$}", " hexdump ", PRELAND_WIDTH);
    for offset in (0..data.len()).step_by(16) {
        for i in 0..16 {
            if offset + i < data.len() {
                print!("{:02x} ", data[offset + i]);
            } else {
                print!("{:02} ", "");
            }
        }

        print!("{:>6}", ' ');

        for i in 0..16 {
            if offset + i < data.len() {
                let c = data[offset + i];
                if c >= 0x20 && c <= 0x7e {
                    print!("{}", c as char);
                } else {
                    print!(".");
                }
            } else {
                print!("{:02} ", "");
            }
        }

        println!("");
    }
    println!("{:-^1$}", " hexdump end ", PRELAND_WIDTH);
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
    exec_with_process(task.clone(), filename, args).expect("can't add task to excutor");
    thread::spawn(task.clone());
}
