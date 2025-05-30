use alloc::{collections::btree_map::BTreeMap, string::String, sync::Arc, vec::Vec};
use devices::PAGE_SIZE;
use executor::AsyncTask;
use libc_types::elf::AuxType;
use polyhal_trap::trapframe::{TrapFrame, TrapFrameArgs};

use crate::{
    consts::{USER_STACK_INIT_SIZE, USER_STACK_TOP},
    tasks::MemType,
};

use super::UserTask;

pub fn init_task_stack(
    user_task: Arc<UserTask>,
    args: Vec<String>,
    base: usize,
    path: &str,
    entry_point: usize,
    ph_count: usize,
    ph_entry_size: usize,
    ph_addr: usize,
    heap_bottom: usize,
) {
    // map stack
    user_task.frame_alloc(
        va!(USER_STACK_TOP - USER_STACK_INIT_SIZE),
        MemType::Stack,
        USER_STACK_INIT_SIZE / PAGE_SIZE,
    );
    log::debug!(
        "[task {}] entry: {:#x}",
        user_task.get_task_id(),
        base + entry_point
    );
    user_task.inner_map(|inner| {
        inner.heap = heap_bottom;
        inner.entry = base + entry_point;
    });

    let mut tcb = user_task.tcb.write();

    tcb.cx = TrapFrame::new();
    tcb.cx[TrapFrameArgs::SP] = USER_STACK_TOP; // stack top;
    tcb.cx[TrapFrameArgs::SEPC] = base + entry_point;

    drop(tcb);

    // push stack
    let envp = vec![
        "LD_LIBRARY_PATH=/",
        "PS1=\x1b[1m\x1b[32mByteOS\x1b[0m:\x1b[1m\x1b[34m\\w\x1b[0m\\$ \0",
        "PATH=/:/bin:/usr/bin",
        "UB_BINDIR=./",
    ];
    let envp: Vec<usize> = envp
        .into_iter()
        .rev()
        .map(|x| user_task.push_str(x))
        .collect();
    let args: Vec<usize> = args
        .into_iter()
        .rev()
        .map(|x| user_task.push_str(&x))
        .collect();

    let random_ptr = user_task.push_arr(&[0u8; 16]);
    let mut auxv = BTreeMap::new();
    auxv.insert(AuxType::Platform, user_task.push_str("riscv"));
    auxv.insert(AuxType::ExecFn, user_task.push_str(path));
    auxv.insert(AuxType::Phnum, ph_count);
    auxv.insert(AuxType::PageSize, PAGE_SIZE);
    auxv.insert(AuxType::Entry, base + entry_point);
    auxv.insert(AuxType::Phent, ph_entry_size);
    auxv.insert(AuxType::Phdr, base + ph_addr);
    auxv.insert(AuxType::GID, 0);
    auxv.insert(AuxType::EGID, 0);
    auxv.insert(AuxType::UID, 0);
    auxv.insert(AuxType::EUID, 0);
    auxv.insert(AuxType::Secure, 0);
    auxv.insert(AuxType::Random, random_ptr);

    // auxv top
    user_task.push(0);
    // TODO: push auxv
    auxv.iter().for_each(|(key, v)| {
        user_task.push(*v);
        user_task.push(*key as usize);
    });

    user_task.push(0);
    envp.iter().for_each(|x| user_task.push(*x));
    user_task.push(0);
    args.iter().for_each(|x| user_task.push(*x));
    user_task.push(args.len());
}
