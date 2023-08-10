#![no_std]
#![feature(used_with_arg)]

extern crate alloc;

#[macro_use]
extern crate log;

use alloc::sync::Arc;
use cv1811_sd::clk_en;
use devices::{device::{Driver, DeviceType, BlkDriver}, BLK_DEVICES, DRIVER_REGS, driver_define};
use fdt::node::FdtNode;

pub struct CvSd;

unsafe impl Sync for CvSd {}
unsafe impl Send for CvSd {}

impl Driver for CvSd {
    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    fn get_id(&self) -> &str {
        "cvitek,sd"
    }

    fn as_blk(self: Arc<Self>) -> Option<Arc<dyn BlkDriver>> {
        Some(self.clone())
    }
}

impl BlkDriver for CvSd {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        clk_en(true);
        cv1811_sd::read_block(block_id as _, buf).expect("can't read block by using CvSd");
        clk_en(false);
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        // unimplemented!("cv sd write");
        clk_en(true);
        cv1811_sd::write_block(block_id as _, buf).expect("can't write block by using CvSd");
        clk_en(false);
    }
}

pub fn init_rtc(_node: &FdtNode) {
    let blk = CvSd;
    cv1811_sd::init().expect("init with err");
    BLK_DEVICES.lock().push(Arc::new(blk));
    info!("Initailize virtio-block device");
}

driver_define!("cvitek,mars-sd", {
    DRIVER_REGS.lock().insert("cvitek,mars-sd", init_rtc);
    None
});
