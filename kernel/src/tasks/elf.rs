use alloc::{collections::BTreeMap, string::String, sync::Arc, vec::Vec};
use devices::PAGE_SIZE;
use executor::AsyncTask;
use libc_types::elf::AuxType;
use log::warn;
use polyhal::va;
use polyhal_trap::trapframe::{TrapFrame, TrapFrameArgs};
use syscalls::Errno;
use xmas_elf::{
    program::Type,
    sections::SectionData,
    symbol_table::{DynEntry64, Entry},
    ElfFile,
};

use crate::consts::USER_STACK_TOP;
use crate::{consts::USER_STACK_INIT_SIZE, tasks::memset::MemType};

use super::task::UserTask;

pub trait ElfExtra {
    fn get_ph_addr(&self) -> Result<u64, Errno>;
    fn dynsym(&self) -> Result<&[DynEntry64], &'static str>;
    fn relocate(&self, base: usize) -> Result<usize, &str>;
}

impl ElfExtra for ElfFile<'_> {
    // 获取elf加载需要的内存大小
    fn get_ph_addr(&self) -> Result<u64, Errno> {
        if let Some(phdr) = self
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Phdr))
        {
            // if phdr exists in program header, use it
            Ok(phdr.virtual_addr())
        } else if let Some(elf_addr) = self
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Load) && ph.offset() == 0)
        {
            // otherwise, check if elf is loaded from the beginning, then phdr can be inferred.
            Ok(elf_addr.virtual_addr() + self.header.pt2.ph_offset())
        } else {
            warn!("elf: no phdr found, tls might not work");
            Err(Errno::EBADF)
        }
    }

    fn dynsym(&self) -> Result<&[DynEntry64], &'static str> {
        match self
            .find_section_by_name(".dynsym")
            .ok_or(".dynsym not found")?
            .get_data(self)
            .map_err(|_| "corrupted .dynsym")?
        {
            SectionData::DynSymbolTable64(dsym) => Ok(dsym),
            _ => Err("bad .dynsym"),
        }
    }

    fn relocate(&self, base: usize) -> Result<usize, &str> {
        let data = self
            .find_section_by_name(".rela.dyn")
            .ok_or(".rela.dyn not found")?
            .get_data(self)
            .map_err(|_| "corrupted .rela.dyn")?;
        let entries = match data {
            SectionData::Rela64(entries) => entries,
            _ => return Err("bad .rela.dyn"),
        };
        let dynsym = self.dynsym()?;
        for entry in entries.iter() {
            const REL_GOT: u32 = 6;
            const REL_PLT: u32 = 7;
            const REL_RELATIVE: u32 = 8;
            const R_RISCV_64: u32 = 2;
            const R_RISCV_RELATIVE: u32 = 3;
            const R_AARCH64_RELATIVE: u32 = 0x403;
            const R_AARCH64_GLOBAL_DATA: u32 = 0x401;

            match entry.get_type() {
                REL_GOT | REL_PLT | R_RISCV_64 | R_AARCH64_GLOBAL_DATA => {
                    let dynsym = &dynsym[entry.get_symbol_table_index() as usize];
                    if dynsym.shndx() == 0 {
                        let name = dynsym.get_name(self)?;
                        panic!("need to find symbol: {:?}", name);
                    } else {
                        base + dynsym.value() as usize
                    };
                }
                REL_RELATIVE | R_RISCV_RELATIVE | R_AARCH64_RELATIVE => {}
                t => unimplemented!("unknown type: {}", t),
            }
        }
        // panic!("STOP");
        Ok(base)
    }
}

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
    user_task.push_num(0);
    // TODO: push auxv
    auxv.iter().for_each(|(key, v)| {
        user_task.push_num(*v);
        user_task.push_num(*key as usize);
    });

    user_task.push_num(0);
    envp.iter().for_each(|x| {
        user_task.push_num(*x);
    });
    user_task.push_num(0);
    args.iter().for_each(|x| {
        user_task.push_num(*x);
    });
    user_task.push_num(args.len());
}
