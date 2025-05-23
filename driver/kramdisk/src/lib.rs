#![no_std]
#![feature(used_with_arg)]

use core::{
    arch::global_asm,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
};

extern crate alloc;

use alloc::sync::Arc;
use devices::{
    device::{BlkDriver, DeviceType, Driver},
    driver_define,
};
use log::info;

// 虚拟IO设备
pub struct RamDiskBlock {
    start: usize,
    size: usize,
}

impl Driver for RamDiskBlock {
    fn get_id(&self) -> &str {
        "kramdisk"
    }

    fn get_device_wrapper(self: Arc<Self>) -> DeviceType {
        DeviceType::BLOCK(self.clone())
    }
}

impl BlkDriver for RamDiskBlock {
    fn read_blocks(&self, sector_offset: usize, buf: &mut [u8]) {
        assert_eq!(buf.len() % 0x200, 0);
        if (sector_offset * 0x200 + buf.len()) >= self.size {
            panic!("can't out of ramdisk range")
        };
        unsafe {
            buf.copy_from_slice(
                slice_from_raw_parts((self.start + sector_offset * 0x200) as *const u8, buf.len())
                    .as_ref()
                    .expect("can't deref ptr in the Ramdisk"),
            );
        }
    }

    fn write_blocks(&self, sector_idx: usize, buf: &[u8]) {
        assert_eq!(buf.len() % 0x200, 0);
        if (sector_idx * 0x200 + buf.len()) >= self.size {
            panic!("can't out of ramdisk range")
        };
        unsafe {
            slice_from_raw_parts_mut((self.start + sector_idx * 0x200) as *mut u8, buf.len())
                .as_mut()
                .expect("can't deref ptr in the ramdisk")
                .copy_from_slice(buf);
        }
    }

    fn capacity(&self) -> usize {
        self.size
    }
}

global_asm!(include_str!(concat!(env!("OUT_DIR"), "/inc.S")));

driver_define!({
    extern "C" {
        fn ramdisk_start();
        fn ramdisk_end();
    }
    info!(
        "ramdisk range: {:#x} - {:#x}",
        ramdisk_start as usize, ramdisk_end as usize
    );
    Some(Arc::new(RamDiskBlock {
        start: ramdisk_start as _,
        size: ramdisk_end as usize - ramdisk_start as usize,
    }))
});
