use super::SysResult;
use crate::syscall::types::mm::map_mprot_to_flags;
use crate::tasks::{MemArea, MemType};
use crate::user::UserTaskContainer;
use crate::utils::useref::UserRef;
use devices::PAGE_SIZE;
use libc_types::mman::{MSyncFlags, MapFlags, MmapProt};
use log::debug;
use polyhal::VirtAddr;
use runtime::frame::alignup;
use syscalls::Errno;

// The high 25bits in sv39 should be the same as bit 38.
const MAP_AREA_START: usize = 0x2_0000_0000;

impl UserTaskContainer {
    pub async fn sys_brk(&self, addr: usize) -> SysResult {
        debug!("sys_brk @ new: {:#x} old: {:#x}", addr, self.task.heap());
        match addr {
            0 => Ok(self.task.heap()),
            _ => Ok(self.task.sbrk(addr)),
        }
    }

    pub async fn sys_mmap(
        &self,
        start: usize,
        mut len: usize,
        prot: usize,
        flags: usize,
        fd: usize,
        off: usize,
    ) -> SysResult {
        let flags = MapFlags::from_bits_truncate(flags as _);
        let prot = MmapProt::from_bits_truncate(prot as _);
        len = alignup(len, PAGE_SIZE);
        debug!(
            "[task {}] sys_mmap @ start: {:#x}, len: {:#x}, prot: {:?}, flags: {:?}, fd: {}, offset: {}",
            self.tid, start, len, prot, flags, fd as isize, off
        );
        let file = self.task.get_fd(fd);

        let addr = self.task.get_last_free_addr();

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

        if flags.contains(MapFlags::FIXED) {
            let overlaped = self
                .task
                .pcb
                .lock()
                .memset
                .overlapping(addr.raw(), addr.raw() + len);
            if overlaped {
                self.task.pcb.lock().memset.sub_area(
                    addr.raw(),
                    addr.raw() + len,
                    &self.task.page_table,
                );
            }
        } else if self
            .task
            .pcb
            .lock()
            .memset
            .overlapping(addr.raw(), addr.raw() + len)
        {
            return Err(Errno::EINVAL);
        }

        if flags.contains(MapFlags::SHARED) {
            match &file {
                Some(file) => self
                    .task
                    .map_frames(
                        addr,
                        MemType::ShareFile,
                        (len + PAGE_SIZE - 1) / PAGE_SIZE,
                        Some(file.get_bare_file()),
                        off,
                        usize::from(addr),
                        len,
                    )
                    .ok_or(Errno::EFAULT)?,
                None => {
                    let paddr = self
                        .task
                        .frame_alloc(addr, MemType::Shared, len.div_ceil(PAGE_SIZE))
                        .ok_or(Errno::EFAULT)?;

                    for i in 0..(len + PAGE_SIZE - 1) / PAGE_SIZE {
                        self.task.map(
                            paddr + i * PAGE_SIZE,
                            addr + i * PAGE_SIZE,
                            map_mprot_to_flags(prot),
                        );
                    }
                    paddr
                }
            };
        } else if file.is_some() {
            self.task
                .frame_alloc(addr, MemType::Mmap, len.div_ceil(PAGE_SIZE))
                .ok_or(Errno::EFAULT)?;
        } else {
            self.task.pcb.lock().memset.push(MemArea {
                mtype: MemType::Mmap,
                mtrackers: vec![],
                file: None,
                offset: 0,
                start: addr.raw(),
                len,
            });
        };

        if let Some(file) = file {
            let buffer = UserRef::<u8>::from(addr).slice_mut_with_len(len);
            file.readat(off, buffer)?;
        }
        Ok(addr.into())
    }

    pub async fn sys_munmap(&self, start: usize, len: usize) -> SysResult {
        debug!("sys_munmap @ start: {:#x}, len: {:#x}", start, len);
        self.task.inner_map(|pcb| {
            pcb.memset
                .sub_area(start, start + len, &self.task.page_table);
        });
        Ok(0)
    }

    pub async fn sys_mprotect(&self, start: usize, len: usize, prot: u32) -> SysResult {
        let prot = MmapProt::from_bits(prot).ok_or(Errno::EINVAL)?;
        debug!(
            "sys_mprotect @ start: {:#x}, len: {:#x}, prot: {:?}",
            start, len, prot
        );
        // Err(LinuxError::EPERM)
        Ok(0)
    }

    pub async fn sys_msync(&self, addr: usize, len: usize, flags: u32) -> SysResult {
        let flags = MSyncFlags::from_bits_truncate(flags);
        debug!(
            "sys_msync @ addr: {:#x} len: {:#x} flags: {:?}",
            addr, len, flags
        );
        // use it temporarily
        Ok(0)
    }
}
