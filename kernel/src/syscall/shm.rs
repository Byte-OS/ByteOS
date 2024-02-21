use core::ops::Add;

use alloc::{sync::Arc, vec::Vec};
use arch::{MappingFlags, VirtAddr, VirtPage, PAGE_SIZE};
use executor::shm::{MapedSharedMemory, SharedMemory, SHARED_MEMORY};
use frame_allocator::{ceil_div, frame_alloc_much, FrameTracker};
use log::debug;

use crate::user::UserTaskContainer;

use super::{consts::LinuxError, SysResult};

// #define IPC_CREAT  01000
// #define IPC_EXCL   02000
// #define IPC_NOWAIT 04000

impl UserTaskContainer {
    pub async fn sys_shmget(&self, mut key: usize, size: usize, shmflg: usize) -> SysResult {
        debug!(
            "sys_shmget @ key: {}, size: {}, shmflg: {:#o}",
            key, size, shmflg
        );
        if key == 0 {
            key = SHARED_MEMORY.lock().keys().cloned().max().unwrap_or(0) + 1;
        }
        let mem = SHARED_MEMORY.lock().get(&key).cloned();
        if mem.is_some() {
            return Ok(key);
        }
        if shmflg & 01000 > 0 {
            let shm: Vec<Arc<FrameTracker>> = frame_alloc_much(ceil_div(size, PAGE_SIZE))
                .expect("can't alloc page in shm")
                .into_iter()
                .map(Arc::new)
                .collect();
            SHARED_MEMORY
                .lock()
                .insert(key, Arc::new(SharedMemory::new(shm)));
            return Ok(key);
        }
        Err(LinuxError::ENOENT)
    }

    pub async fn sys_shmat(&self, shmid: usize, shmaddr: usize, shmflg: usize) -> SysResult {
        debug!(
            "sys_shmat @ shmid: {}, shmaddr: {}, shmflg: {:#o}",
            shmid, shmaddr, shmflg
        );
        let addr = self.task.get_last_free_addr();

        let addr = if shmaddr == 0 {
            if usize::from(addr) >= 0x4000_0000 {
                addr
            } else {
                VirtAddr::from(0x4000_0000)
            }
        } else {
            VirtAddr::new(shmaddr)
        };
        let vpn = VirtPage::from(addr);
        let trackers = SHARED_MEMORY.lock().get(&shmid).cloned();
        if trackers.is_none() {
            return Err(LinuxError::ENOENT);
        }
        trackers
            .as_ref()
            .unwrap()
            .trackers
            .iter()
            .enumerate()
            .for_each(|(i, x)| {
                debug!("map {:?} @ {:?}", vpn.add(i), x.0);
                self.task.map(x.0, vpn.add(i), MappingFlags::URWX);
            });
        let size = trackers.as_ref().unwrap().trackers.len() * PAGE_SIZE;
        self.task.pcb.lock().shms.push(MapedSharedMemory {
            key: shmid,
            mem: trackers.unwrap(),
            start: addr.addr(),
            size,
        });
        Ok(addr.addr())
    }

    pub async fn sys_shmctl(&self, shmid: usize, cmd: usize, arg: usize) -> SysResult {
        debug!("sys_shmctl @ shmid: {}, cmd: {}, arg: {}", shmid, cmd, arg);

        if cmd == 0 {
            // SHARED_MEMORY.lock().remove(&shmid);
            if let Some(map) = SHARED_MEMORY.lock().get_mut(&shmid) {
                *map.deleted.lock() = true;
            }
            return Ok(0);
        }
        Err(LinuxError::EPERM)
    }
}
