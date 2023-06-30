use core::ops::Add;

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use arch::{PAGE_SIZE, VirtAddr, VirtPage, PTEFlags};
use executor::{MapTrack, current_user_task};
use frame_allocator::{FrameTracker, frame_alloc_much, ceil_div};
use log::debug;
use sync::Mutex;

use super::consts::LinuxError;

// #define IPC_CREAT  01000
// #define IPC_EXCL   02000
// #define IPC_NOWAIT 04000

#[derive(Clone)]
pub struct SharedMemory {
    trackers: Vec<Arc<FrameTracker>>
}

pub struct MapedSharedMemory {
    key: usize,
    mem: Arc<SharedMemory>
}

impl Drop for MapedSharedMemory {
    fn drop(&mut self) {
        // self.mem.trackers.remove(self.key);
        if Arc::strong_count(&self.mem) == 1 {
            SHARED_MEMORY.lock().remove(&self.key);
        }
    }
}

pub static SHARED_MEMORY: Mutex<BTreeMap<usize, Arc<SharedMemory>>> = Mutex::new(BTreeMap::new());

pub async fn sys_shmget(mut key: usize, size: usize, shmflg: usize) -> Result<usize, LinuxError> {
    debug!("sys_shmget @ key: {}, size: {}, shmflg: {:#o}", key, size, shmflg);
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
        SHARED_MEMORY.lock().insert(key, Arc::new(SharedMemory {
            trackers: shm
        }));
        return Ok(key);
    }
    Err(LinuxError::ENOENT)
}

pub async fn sys_shmat(shmid: usize, shmaddr: usize, shmflg: usize) -> Result<usize, LinuxError> {
    debug!("sys_shmat @ shmid: {}, shmaddr: {}, shmflg: {:#o}", shmid, shmaddr, shmflg);
    let task = current_user_task();
    
    let addr = task.get_last_free_addr();

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
    log::error!("virtual: {:?}  addr: {:?}", vpn, addr);
    let trackers = SHARED_MEMORY.lock().get(&shmid).cloned();
    if trackers.is_none() {
        return Err(LinuxError::ENOENT);
    }
    trackers.unwrap().trackers.iter().enumerate().for_each(|(i, x)| {
        debug!("map {:?} @ {:?}", vpn.add(i), x.0);
        task.map(x.0, vpn.add(i), PTEFlags::UVRWX);
    });
    // let addr = 
    // Err(LinuxError::EPERM)
    Ok(addr.addr())
}

pub async fn sys_shmctl(shmid: usize, cmd: usize, arg: usize) -> Result<usize, LinuxError> {
    debug!("sys_shmctl @ shmid: {}, cmd: {}, arg: {}", shmid, cmd, arg);

    if cmd == 0 {
        SHARED_MEMORY.lock().remove(&shmid);
        return Ok(0);
    }
    Err(LinuxError::EPERM)
}