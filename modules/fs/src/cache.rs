use core::cmp;

use alloc::{collections::BTreeMap, string::String, vec::Vec};
use arch::{ppn_c, PAGE_SIZE};
use frame_allocator::{ceil_div, frame_alloc_much, FrameTracker};
use log::debug;
use sync::Mutex;

use crate::mount::open;

pub struct CacheItem {
    data: &'static mut [u8],
    _trackers: Vec<FrameTracker>,
}

static CACHE_TABLE: Mutex<BTreeMap<String, CacheItem>> = Mutex::new(BTreeMap::new());

/// cached(filename: &str) 判断文件是否被缓存
/// filename 应包含文件路径，如: 根目录下的 entry-static.exe 为 /entry-static.exe
pub fn cached(filename: &str) -> bool {
    debug!("get cached file: {}", filename);
    CACHE_TABLE.lock().contains_key(filename)
}

/// cache_read(filename: &str, buffer: &mut [u8], offset: usize) -> usize
/// 从 cache_table 读取文件，需要先使用 cached(filename: &str) 确定文件已经被缓存
pub fn cache_read(filename: &str, buffer: &mut [u8], offset: usize) -> usize {
    let cache_table = CACHE_TABLE.lock();
    let cache_file = cache_table.get(filename).unwrap();

    let len = buffer.len();
    let rlen = cmp::min(len - offset, buffer.len());
    buffer[..rlen].copy_from_slice(&cache_file.data[offset..offset + rlen]);
    rlen
}

/// cache_file(path: &str) 缓存文件，
pub fn cache_file(path: &str) {
    if let Ok(file) = open(path) {
        let len = file.metadata().expect("can't get file metadata").size;
        let count = ceil_div(len, PAGE_SIZE);
        let trackers = frame_alloc_much(count).expect("can't alloc frame trackers from cache");
        let buffer = unsafe {
            let ppn = trackers[0].0;
            core::slice::from_raw_parts_mut(ppn_c(ppn).to_addr() as *mut u8, len)
        };
        let mut cache_item = CacheItem {
            data: buffer,
            _trackers: trackers,
        };
        file.read(&mut cache_item.data).expect("can't read file");
        CACHE_TABLE.lock().insert(String::from(path), cache_item);
        info!("cache file: {}", path);
    } else {
        info!("cache file: {}  failed", path);
    }
}

/// init() 初始化缓存表
pub fn init() {
    cache_file("/entry-static.exe");
    cache_file("/entry-dynamic.exe");
    cache_file("/runtest.exe");
    cache_file("/libc.so");
    cache_file("/busybox");
}
