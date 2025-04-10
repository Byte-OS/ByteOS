use super::UserTask;
use crate::{
    consts::USER_DYN_ADDR,
    tasks::{
        elf::{init_task_stack, ElfExtra},
        MapTrack, MemArea, MemType,
    },
    utils::vfs::from_vfs,
};
use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use async_recursion::async_recursion;
use core::ops::{Add, Mul};
use devices::{frame_alloc_much, FrameTracker, PAGE_SIZE};
use fs::{
    dentry::{dentry_open, dentry_root},
    OpenFlags,
};
use polyhal::MappingFlags;
use sync::Mutex;
use syscalls::Errno;
use xmas_elf::program::{SegmentData, Type};

pub struct TaskCacheTemplate {
    name: String,
    entry: usize,
    maps: Vec<MemArea>,
    base: usize,
    heap_bottom: usize,
    ph_count: usize,
    ph_entry_size: usize,
    ph_addr: usize,
}
pub static TASK_CACHES: Mutex<Vec<TaskCacheTemplate>> = Mutex::new(Vec::new());

pub fn cache_task_template(path: &str) -> Result<(), Errno> {
    let file = dentry_open(dentry_root(), path, OpenFlags::O_RDONLY)
        .map_err(from_vfs)?
        .node
        .clone();
    let file_size = file.metadata().unwrap().size;
    let frame_paddr = frame_alloc_much(file_size.div_ceil(PAGE_SIZE));
    let buffer = frame_paddr.as_ref().unwrap()[0].slice_mut_with_len(file_size);
    let rsize = file.readat(0, buffer).map_err(from_vfs)?;
    assert_eq!(rsize, file_size);
    // flush_dcache_range();
    // 读取elf信息
    if let Ok(elf) = xmas_elf::ElfFile::new(&buffer) {
        let elf_header = elf.header;

        let entry_point = elf.header.pt2.entry_point() as usize;
        // this assert ensures that the file is elf file.
        assert_eq!(
            elf_header.pt1.magic,
            [0x7f, 0x45, 0x4c, 0x46],
            "invalid elf!"
        );

        // check if it is libc, dlopen, it needs recurit.
        let header = elf
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Interp));

        if let Some(_header) = header {
            unimplemented!("can't cache dynamic file.");
        }

        // 获取程序所有段之后的内存，4K 对齐后作为堆底
        let heap_bottom = elf
            .program_iter()
            .map(|x| (x.virtual_addr() + x.mem_size()) as usize)
            .max()
            .unwrap()
            .div_ceil(PAGE_SIZE)
            .mul(PAGE_SIZE);

        let base = elf.relocate(USER_DYN_ADDR).unwrap_or(0);
        let mut maps = Vec::new();

        // map sections.
        elf.program_iter()
            .filter(|x| x.get_type().unwrap() == xmas_elf::program::Type::Load)
            .for_each(|ph| {
                let file_size = ph.file_size() as usize;
                let mem_size = ph.mem_size() as usize;
                let offset = ph.offset() as usize;
                let virt_addr = base + ph.virtual_addr() as usize;
                let vpn = virt_addr / PAGE_SIZE;

                let page_count = (virt_addr + mem_size).div_ceil(PAGE_SIZE) - vpn;
                let pages: Vec<Arc<FrameTracker>> = frame_alloc_much(page_count)
                    .expect("can't alloc in cache task template")
                    .into_iter()
                    .map(|x| Arc::new(x))
                    .collect();
                let ppn_space = pages[0]
                    .add(virt_addr % PAGE_SIZE)
                    .slice_mut_with_len(file_size);

                ppn_space.copy_from_slice(&buffer[offset..offset + file_size]);

                maps.push(MemArea {
                    mtype: MemType::CodeSection,
                    mtrackers: pages
                        .into_iter()
                        .enumerate()
                        .map(|(i, x)| MapTrack {
                            vaddr: va!((vpn + i) * PAGE_SIZE),
                            tracker: x,
                            rwx: 0,
                        })
                        .collect(),
                    file: None,
                    offset: 0,
                    start: vpn * PAGE_SIZE,
                    len: page_count * PAGE_SIZE,
                })
            });
        TASK_CACHES.lock().push(TaskCacheTemplate {
            name: path.to_string(),
            entry: entry_point,
            maps,
            base,
            heap_bottom,
            ph_count: elf_header.pt2.ph_count() as _,
            ph_entry_size: elf_header.pt2.ph_entry_size() as _,
            ph_addr: elf.get_ph_addr().unwrap_or(0) as _,
        });
    }
    Ok(())
}

#[async_recursion(Sync)]
pub async fn exec_with_process(
    task: Arc<UserTask>,
    path: String,
    args: Vec<String>,
    envp: Vec<String>,
) -> Result<Arc<UserTask>, Errno> {
    // copy args, avoid free before pushing.
    let path = String::from(path);
    let user_task = task.clone();
    user_task.pcb.lock().memset.clear();
    user_task.page_table.restore();
    user_task.page_table.change();

    let caches = TASK_CACHES.lock();
    if let Some(cache_task) = caches.iter().find(|x| x.name == path) {
        init_task_stack(
            user_task.clone(),
            args,
            cache_task.base,
            &path,
            cache_task.entry,
            cache_task.ph_count,
            cache_task.ph_entry_size,
            cache_task.ph_addr,
            cache_task.heap_bottom,
        );

        for area in &cache_task.maps {
            user_task.inner_map(|pcb| {
                pcb.memset
                    .sub_area(area.start, area.start + area.len, &user_task.page_table);
                pcb.memset.push(area.clone());
            });
            for mtracker in area.mtrackers.iter() {
                user_task.map(mtracker.tracker.0, mtracker.vaddr, MappingFlags::URX);
            }
        }
        Ok(user_task)
    } else {
        drop(caches);
        let file = dentry_open(dentry_root(), &path, OpenFlags::O_RDONLY)
            .map_err(from_vfs)?
            .node
            .clone();
        debug!("file: {:#x?}", file.metadata().unwrap());
        let file_size = file.metadata().unwrap().size;
        let frame_ppn = frame_alloc_much(file_size.div_ceil(PAGE_SIZE));
        let buffer = frame_ppn.as_ref().unwrap()[0].slice_mut_with_len(file_size);
        let rsize = file.readat(0, buffer).map_err(from_vfs)?;
        assert_eq!(rsize, file_size);
        // flush_dcache_range();
        // 读取elf信息
        let elf = if let Ok(elf) = xmas_elf::ElfFile::new(&buffer) {
            elf
        } else {
            let mut new_args = vec!["busybox".to_string(), "sh".to_string()];
            args.iter().for_each(|x| new_args.push(x.clone()));
            return exec_with_process(task, String::from("busybox"), new_args, envp).await;
        };
        let elf_header = elf.header;

        let entry_point = elf.header.pt2.entry_point() as usize;
        // this assert ensures that the file is elf file.
        assert_eq!(
            elf_header.pt1.magic,
            [0x7f, 0x45, 0x4c, 0x46],
            "invalid elf!"
        );
        // WARRNING: this convert async task to user task.
        let user_task = task.clone();

        // check if it is libc, dlopen, it needs recurit.
        let header = elf
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Interp));
        if let Some(header) = header {
            if let Ok(SegmentData::Undefined(_data)) = header.get_data(&elf) {
                drop(frame_ppn);
                let mut new_args = vec![String::from("libc.so")];
                new_args.extend(args);
                return exec_with_process(task, new_args[0].clone(), new_args, envp).await;
            }
        }

        // 获取程序所有段之后的内存，4K 对齐后作为堆底
        let heap_bottom = elf
            .program_iter()
            .map(|x| (x.virtual_addr() + x.mem_size()) as usize)
            .max()
            .unwrap()
            .div_ceil(PAGE_SIZE)
            .mul(PAGE_SIZE);

        let base = elf.relocate(USER_DYN_ADDR).unwrap_or(0);
        init_task_stack(
            user_task.clone(),
            args,
            base,
            &path,
            entry_point,
            elf_header.pt2.ph_count() as usize,
            elf_header.pt2.ph_entry_size() as usize,
            elf.get_ph_addr().unwrap_or(0) as usize,
            heap_bottom,
        );

        // map sections.
        elf.program_iter()
            .filter(|x| x.get_type().unwrap() == xmas_elf::program::Type::Load)
            .for_each(|ph| {
                let file_size = ph.file_size() as usize;
                let mem_size = ph.mem_size() as usize;
                let offset = ph.offset() as usize;
                let virt_addr = base + ph.virtual_addr() as usize;
                let vpn = virt_addr / PAGE_SIZE;

                let page_count = (virt_addr + mem_size).div_ceil(PAGE_SIZE) - vpn;
                let ppn_start =
                    user_task.frame_alloc(va!(virt_addr).floor(), MemType::CodeSection, page_count);
                let page_space = va!(virt_addr).slice_mut_with_len(file_size);
                let ppn_space = ppn_start
                    .expect("not have enough memory")
                    .add(virt_addr % PAGE_SIZE)
                    .slice_mut_with_len(file_size);

                page_space.copy_from_slice(&buffer[offset..offset + file_size]);
                assert_eq!(ppn_space, page_space);
                assert_eq!(&buffer[offset..offset + file_size], ppn_space);
                assert_eq!(&buffer[offset..offset + file_size], page_space);
            });
        Ok(user_task)
    }
}
