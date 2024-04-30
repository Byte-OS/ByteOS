use alloc::{sync::Arc, vec::Vec};
use polyhal::addr::VirtPage;
use polyhal::{pagetable::PageTable, PAGE_SIZE};
use core::{
    cmp::min,
    fmt::Debug,
    ops::{Deref, DerefMut},
};
use frame_allocator::FrameTracker;
use fs::File;

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

    pub fn overlapping(&self, start: usize, end: usize) -> bool {
        self.0.iter().find(|x| x.overlapping(start, end)).is_some()
    }

    pub fn sub_area(&mut self, start: usize, end: usize, pt: &PageTable) {
        let mut new_set = Vec::new();
        self.0.retain_mut(|area| {
            let res = area.sub(start, end, pt);
            if let Some(new_area) = res {
                new_set.push(new_area);
            }
            area.len != 0
        });
        self.0.extend(new_set);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum MemType {
    CodeSection,
    Stack,
    Mmap,
    Shared,
    ShareFile,
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
    pub offset: usize,
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
    /// Check the memory is overlapping.
    pub fn overlapping(&self, start: usize, end: usize) -> bool {
        let self_end = self.start + self.len;
        let res =
            !((start <= self.start && end <= self.start) || (start >= self_end && end >= self_end));
        res
    }

    /// write page to file
    pub fn write_page(&self, mtracker: &MapTrack) {
        assert!(self.file.is_some());
        if let Some(file) = &self.file {
            let offset = mtracker.vpn.to_addr() + self.offset - self.start;
            file.writeat(offset, mtracker.tracker.0.get_buffer())
                .expect("can't write data back to mapped file.");
        }
    }

    /// Sub the memory from this memory area.
    /// the return value indicates whether the memory is splited.
    pub fn sub(&mut self, start: usize, end: usize, pt: &PageTable) -> Option<MemArea> {
        if !self.overlapping(start, end) {
            return None;
        }
        let range = self.start..self.start + self.len;
        let jrange = start..end;

        if range.contains(&start) && range.contains(&end) {
            self.len = start - self.start;
            let new_area_range = end..range.end;

            if let Some(_file) = &self.file {
                self.mtrackers
                    .iter()
                    .filter(|x| jrange.contains(&x.vpn.to_addr()))
                    .for_each(|x| {
                        self.write_page(x);
                    });
            };
            // drop the sub memory area pages.
            self.mtrackers
                .retain(|x| !new_area_range.contains(&x.vpn.to_addr()));
            return Some(MemArea {
                mtype: self.mtype,
                mtrackers: self
                    .mtrackers
                    .extract_if(|x| new_area_range.contains(&x.vpn.to_addr()))
                    .collect(),
                file: self.file.clone(),
                start: end,
                offset: end - self.start,
                len: new_area_range.len(),
            });
        }

        if jrange.contains(&self.start) && jrange.contains(&range.end) {
            self.len = 0;
            // TIPS: This area will be remove outside this function.
            // So return the None.
            if let Some(_file) = &self.file {
                self.mtrackers
                    .iter()
                    .filter(|x| jrange.contains(&x.vpn.to_addr()))
                    .for_each(|x| {
                        self.write_page(x);
                    });
            };
            self.mtrackers.retain(|x| {
                pt.unmap_page(x.vpn);
                false
            });
            return None;
        }

        if range.contains(&start) {
            // self.len = cmp::min(start - self.start, self.len);
            self.len = start - self.start;
        } else if jrange.contains(&self.start) {
            self.len = self.start + self.len - end;
            self.start = end;
        }
        if let Some(_file) = &self.file {
            self.mtrackers
                .iter()
                .filter(|x| jrange.contains(&x.vpn.to_addr()))
                .for_each(|x| {
                    self.write_page(x);
                });
        };
        // drop the sub memory area pages.
        let new_self_rang = self.start..self.start + self.len;
        self.mtrackers
            .extract_if(|x| !new_self_rang.contains(&x.vpn.to_addr()))
            .for_each(|x| {
                pt.unmap_page(x.vpn);
            });
        None
    }

    /// Check the memory area whether contains the specified address.
    pub fn contains(&self, addr: usize) -> bool {
        self.start <= addr && addr < self.start + self.len
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

                    let bytes = &mut tracker.tracker.0.get_buffer()[..wlen];
                    mapfile
                        .writeat(offset, bytes)
                        .expect("can't write data to file at drop");
                }
            }
            _ => {}
        }
    }
}
