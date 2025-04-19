use super::SysResult;
use crate::tasks::{MapedSharedMemory, SharedMemory, SHARED_MEMORY};
use crate::user::UserTaskContainer;
use alloc::{sync::Arc, vec::Vec};
use devices::PAGE_SIZE;
use log::debug;
use polyhal::{va, MappingFlags};
use runtime::frame::{frame_alloc_much, FrameTracker};
use syscalls::Errno;

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
            let shm: Vec<Arc<FrameTracker>> = frame_alloc_much(size.div_ceil(PAGE_SIZE))
                .expect("can't alloc page in shm")
                .into_iter()
                .map(Arc::new)
                .collect();
            SHARED_MEMORY
                .lock()
                .insert(key, Arc::new(SharedMemory::new(shm)));
            return Ok(key);
        }
        Err(Errno::ENOENT)
    }

    pub async fn sys_shmat(&self, shmid: usize, shmaddr: usize, shmflg: usize) -> SysResult {
        debug!(
            "sys_shmat @ shmid: {}, shmaddr: {}, shmflg: {:#o}",
            shmid, shmaddr, shmflg
        );
        let vaddr = self.task.get_last_free_addr();

        let vaddr = if shmaddr == 0 {
            if vaddr >= va!(0x4000_0000) {
                vaddr
            } else {
                va!(0x4000_0000)
            }
        } else {
            va!(shmaddr)
        };
        let trackers = SHARED_MEMORY.lock().get(&shmid).cloned();
        if trackers.is_none() {
            return Err(Errno::ENOENT);
        }
        trackers
            .as_ref()
            .unwrap()
            .trackers
            .iter()
            .enumerate()
            .for_each(|(i, x)| {
                debug!("map {:?} @ {:?}", vaddr.raw() + i * PAGE_SIZE, x.0);
                self.task
                    .map(x.0, vaddr + i * PAGE_SIZE, MappingFlags::URWX);
            });
        let size = trackers.as_ref().unwrap().trackers.len() * PAGE_SIZE;
        self.task.pcb.lock().shms.push(MapedSharedMemory {
            key: shmid,
            mem: trackers.unwrap(),
            start: vaddr.raw(),
            size,
        });
        Ok(vaddr.raw())
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
        Err(Errno::EPERM)
    }
}
