use alloc::{sync::Arc, vec::Vec};
use arch::{VirtPage, PAGE_SIZE};
use core::{
    cmp::min,
    fmt::Debug,
    ops::{Deref, DerefMut},
};
use frame_allocator::{frame_alloc, FrameTracker};
use fs::File;

pub struct MemSetTrackerIteror<'a> {
    value: &'a MemSet,
    area_index: usize,
    inner_index: usize,
}

/// The iter for memset trackers.
impl<'a> Iterator for MemSetTrackerIteror<'a> {
    type Item = &'a MapTrack;

    fn next(&mut self) -> Option<Self::Item> {
        if self.area_index >= self.value.0.len() {
            return None;
        }
        let mem_area = &self.value.0[self.area_index];
        if self.inner_index >= mem_area.mtrackers.len() {
            return None;
        }
        let ans = &mem_area.mtrackers[self.inner_index];
        self.inner_index += 1;

        if self.inner_index >= mem_area.mtrackers.len() {
            self.inner_index = 0;
            self.area_index += 1;
        }
        Some(ans)
    }
}

/// Memory set for storing the memory and its map relation.
#[derive(Debug)]
pub struct MemSet(Vec<MemArea>);

/// Deref for memset, let it iterable
impl Deref for MemSet {
    type Target = Vec<MemArea>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// DerefMut for memset, let it iterable
impl DerefMut for MemSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> MemSet {
    pub fn new(vec: Vec<MemArea>) -> Self {
        Self(vec)
    }

    pub fn trackers_iter(&'a self) -> MemSetTrackerIteror<'a> {
        MemSetTrackerIteror {
            value: self,
            area_index: 0,
            inner_index: 0,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Copy)]
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
    pub rwx: u8,
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
            self.mtrackers.push(MapTrack {
                vpn,
                tracker,
                rwx: 0,
            })
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

                    // let bytes = unsafe {
                    //     core::slice::from_raw_parts_mut(
                    //         ppn_c(tracker.tracker.0).to_addr() as *mut u8,
                    //         wlen as usize,
                    //     )
                    // };
                    let bytes = &mut tracker.tracker.0.get_buffer()[..wlen];
                    // mapfile
                    //     .seek(SeekFrom::SET(offset as usize))
                    //     .expect("can't write data to file");
                    mapfile
                        .writeat(offset, bytes)
                        .expect("can't write data to file at drop");
                }
            }
            _ => {}
        }
    }
}
