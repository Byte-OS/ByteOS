use alloc::{sync::Arc, vec::Vec};
use arch::{ppn_c, VirtPage, PAGE_SIZE};
use core::{cmp::min, fmt::Debug};
use frame_allocator::{frame_alloc, FrameTracker};
use fs::{File, SeekFrom};

#[derive(Clone, PartialEq, Debug)]
pub enum MemType {
    CodeSection,
    Stack,
    Mmap,
    Shared,
    ShareFile,
    Clone,
    PTE,
}

#[derive(Clone)]
pub struct MapTrack {
    pub vpn: VirtPage,
    pub tracker: Arc<FrameTracker>,
}

impl Debug for MapTrack {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "{:#x} -> {:#x}",
            self.vpn.to_addr(),
            self.tracker.0.to_addr()
        ))
    }
}

#[derive(Clone)]
pub struct MemArea {
    pub mtype: MemType,
    pub mtrackers: Vec<MapTrack>,
    pub file: Option<File>,
    pub start: usize,
    pub len: usize,
}

impl Debug for MemArea {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MemArea")
            .field("mtype", &self.mtype)
            .field("mtrackers", &self.mtrackers)
            .field("start", &self.start)
            .field("len", &self.len)
            .finish()
    }
}

impl MemArea {
    pub fn new(mtype: MemType, mtrackers: Vec<MapTrack>) -> Self {
        MemArea {
            mtype,
            mtrackers,
            file: None,
            start: 0,
            len: 0,
        }
    }
    pub fn map(&mut self, vpn: VirtPage, tracker: Arc<FrameTracker>) {
        let finded_tracker = self
            .mtrackers
            .iter_mut()
            .find(|x| x.vpn == vpn && x.vpn.to_addr() != 0);
        if let Some(map_track) = finded_tracker {
            map_track.tracker = tracker;
        } else {
            self.mtrackers.push(MapTrack { vpn, tracker })
        }
    }
    pub fn fork(&self) -> Self {
        match self.mtype {
            MemType::ShareFile | MemType::Shared => self.clone(),
            MemType::PTE => Self::new(MemType::PTE, vec![]),
            _ => {
                let mut res = self.clone();
                for map_track in res.mtrackers.iter_mut() {
                    let tracker = frame_alloc().expect("can't alloc page in fork");
                    tracker.0.copy_value_from_another(map_track.tracker.0);
                    map_track.tracker = Arc::new(tracker);
                }
                res
            }
        }
    }
}

impl Drop for MemArea {
    fn drop(&mut self) {
        match &self.mtype {
            MemType::ShareFile => {
                let start = self.start;
                let len = self.len;
                let mapfile = self.file.clone().unwrap();
                for tracker in &self.mtrackers {
                    if Arc::strong_count(&tracker.tracker) > 1 {
                        continue;
                    }

                    let offset = tracker.vpn.to_addr() - start;
                    let wlen = min(len - offset, PAGE_SIZE);

                    let bytes = unsafe {
                        core::slice::from_raw_parts_mut(
                            ppn_c(tracker.tracker.0).to_addr() as *mut u8,
                            wlen as usize,
                        )
                    };
                    mapfile
                        .seek(SeekFrom::SET(offset as usize))
                        .expect("can't write data to file");
                    mapfile
                        .write(bytes)
                        .expect("can't write data to file at drop");
                }
            }
            _ => {}
        }
    }
}
