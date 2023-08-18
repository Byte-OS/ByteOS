use core::ops::Add;

use arch::{VirtAddr, VirtPage, PAGE_SIZE};
use executor::current_task;
use executor::current_user_task;
use executor::AsyncTask;
use executor::MemArea;
use frame_allocator::ceil_div;
use log::debug;
use vfscore::INodeInterface;

use crate::syscall::consts::from_vfs;
use crate::syscall::consts::MSyncFlags;
use crate::syscall::consts::MapFlags;
use crate::syscall::consts::MmapProt;
use crate::syscall::consts::ProtFlags;
use crate::syscall::consts::UserRef;

use super::consts::LinuxError;

// The high 25bits in sv39 should be the same as bit 38.
const MAP_AREA_START: usize = 0x1_0000_0000;

pub async fn sys_brk(addr: isize) -> Result<usize, LinuxError> {
    debug!("sys_brk @ increment: {:#x}", addr);
    let user_task = current_task().as_user_task().unwrap();
    if addr == 0 {
        Ok(user_task.heap())
    } else {
        debug!("alloc pos: {}", addr - user_task.heap() as isize);
        Ok(user_task.sbrk(addr - user_task.heap() as isize))
    }
}

pub async fn sys_mmap(
    start: usize,
    len: usize,
    prot: usize,
    flags: usize,
    fd: usize,
    off: usize,
) -> Result<usize, LinuxError> {
    let flags = MapFlags::from_bits_truncate(flags as _);
    let prot = MmapProt::from_bits_truncate(prot as _);
    let task = current_user_task();
    debug!(
        "[task {}] sys_mmap @ start: {:#x}, len: {:#x}, prot: {:?}, flags: {:?}, fd: {}, offset: {}",
        task.get_task_id(), start, len, prot, flags, fd as isize, off
    );
    let file = task.get_fd(fd);

    let addr = task.get_last_free_addr();

    let addr = if start == 0 {
        if usize::from(addr) >= MAP_AREA_START {
            addr
        } else {
            VirtAddr::from(MAP_AREA_START)
        }
    } else {
        VirtAddr::new(start)
    };

    if len == 0 {
        return Ok(addr.into());
    }

    if flags.contains(MapFlags::MAP_FIXED) {
        let overlaped = task
            .pcb
            .lock()
            .memset
            .overlapping(addr.addr(), addr.addr() + len);
        if overlaped {
            task.pcb
                .lock()
                .memset
                .sub_area(addr.addr(), addr.addr() + len, task.page_table);
        }
    } else if task
        .pcb
        .lock()
        .memset
        .overlapping(addr.addr(), addr.addr() + len)
    {
        return Err(LinuxError::EINVAL);
    }

    if flags.contains(MapFlags::MAP_SHARED) {
        match &file {
            Some(file) => task
                .map_frames(
                    VirtPage::from_addr(addr.into()),
                    executor::MemType::ShareFile,
                    (len + PAGE_SIZE - 1) / PAGE_SIZE,
                    Some(file.get_bare_file()),
                    off,
                    usize::from(addr),
                    len,
                )
                .ok_or(LinuxError::EFAULT)?,
            None => {
                let ppn = task
                    .frame_alloc(
                        VirtPage::from_addr(addr.into()),
                        executor::MemType::Shared,
                        (len + PAGE_SIZE - 1) / PAGE_SIZE,
                    )
                    .ok_or(LinuxError::EFAULT)?;

                for i in 0..(len + PAGE_SIZE - 1) / PAGE_SIZE {
                    task.map(
                        ppn.add(i),
                        VirtPage::from_addr(addr.into()).add(i),
                        prot.into(),
                    );
                }
                ppn
            }
        };
    } else if file.is_some() {
        task.frame_alloc(
            VirtPage::from_addr(addr.into()),
            executor::MemType::Mmap,
            ceil_div(len, PAGE_SIZE),
        )
        .ok_or(LinuxError::EFAULT)?;
    } else {
        // task.frame_alloc(
        //     VirtPage::from_addr(addr.into()),
        //     executor::MemType::Mmap,
        //     ceil_div(len, PAGE_SIZE),
        // )
        // .ok_or(LinuxError::EFAULT)?;

        task.pcb.lock().memset.push(MemArea {
            mtype: executor::MemType::Mmap,
            mtrackers: vec![],
            file: None,
            offset: 0,
            start: addr.addr(),
            len,
        });
    };

    if let Some(file) = file {
        let buffer = UserRef::<u8>::from(addr).slice_mut_with_len(len);
        file.readat(off, buffer).map_err(from_vfs)?;
    }
    Ok(addr.into())
}

pub async fn sys_munmap(start: usize, len: usize) -> Result<usize, LinuxError> {
    debug!("sys_munmap @ start: {:#x}, len: {:#x}", start, len);
    let task = current_user_task();
    task.inner_map(|pcb| {
        pcb.memset.sub_area(start, start + len, task.page_table);
    });
    Ok(0)
}

pub async fn sys_mprotect(start: usize, len: usize, prot: u32) -> Result<usize, LinuxError> {
    let prot = ProtFlags::from_bits_truncate(prot);
    debug!(
        "sys_mprotect @ start: {:#x}, len: {:#x}, prot: {:?}",
        start, len, prot
    );
    // Err(LinuxError::EPERM)
    Ok(0)
}

pub async fn sys_msync(addr: usize, len: usize, flags: u32) -> Result<usize, LinuxError> {
    let flags = MSyncFlags::from_bits_truncate(flags);
    debug!(
        "sys_msync @ addr: {:#x} len: {:#x} flags: {:?}",
        addr, len, flags
    );
    // use it temporarily
    Ok(0)
}
