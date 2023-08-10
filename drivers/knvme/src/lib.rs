#![no_std]
#![feature(used_with_arg)]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate log;

use alloc::{sync::Arc, vec::Vec};
use arch::{PAGE_SIZE, VIRT_ADDR_START};
use devices::{device::{Driver, DeviceType, BlkDriver}, BLK_DEVICES, driver_define};
use frame_allocator::{frame_alloc_much, FrameTracker};
use nvme_driver::{DmaAllocator, IrqController, NvmeInterface};
use sync::Mutex;

use core::ptr::write_volatile;

static VIRTIO_CONTAINER: Mutex<Vec<FrameTracker>> = Mutex::new(Vec::new());

pub struct DmaAllocatorImpl;
impl DmaAllocator for DmaAllocatorImpl {
    fn dma_alloc(size: usize) -> usize {
        // 申请内存
        debug!("nvme alloc memeory: {}", size);
        let pages =
            frame_alloc_much(size / PAGE_SIZE).expect("can't alloc page in devices/nvme.rs");
        let ppn = pages[0].0;
        VIRTIO_CONTAINER.lock().extend(pages.into_iter());
        ppn.to_addr() | VIRT_ADDR_START
    }

    fn dma_dealloc(addr: usize, size: usize) -> usize {
        debug!("nvme dealloc memory: {}", size);
        VIRTIO_CONTAINER
            .lock()
            .retain(|x| !(addr..addr + size).contains(&x.0.to_addr()));
        0
    }

    fn phys_to_virt(phys: usize) -> usize {
        phys | VIRT_ADDR_START
    }

    fn virt_to_phys(virt: usize) -> usize {
        virt & (!VIRT_ADDR_START)
    }
}

pub struct IrqControllerImpl;

impl IrqController for IrqControllerImpl {
    fn enable_irq(_irq: usize) {}

    fn disable_irq(_irq: usize) {}
}

// 虚拟IO设备
pub struct VirtIOBlock(pub NvmeInterface<DmaAllocatorImpl, IrqControllerImpl>);

impl Driver for VirtIOBlock {
    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    fn get_id(&self) -> &str {
        "nvme"
    }

    fn as_blk(self: Arc<Self>) -> Option<Arc<dyn BlkDriver>> {
        Some(self.clone())
    }
}

impl BlkDriver for VirtIOBlock {
    fn read_block(&self, sector_offset: usize, buf: &mut [u8]) {
        // 读取文件
        self.0.read_block(sector_offset, buf)
    }

    fn write_block(&self, sector_offset: usize, buf: &[u8]) {
        self.0.write_block(sector_offset, buf)
    }
}

// config pci
pub fn config_pci() {
    let ptr = (VIRT_ADDR_START | 0x30008010) as *mut u32;
    unsafe {
        write_volatile(ptr, 0xffffffff);
    }
    let ptr = (VIRT_ADDR_START | 0x30008010) as *mut u32;
    unsafe {
        write_volatile(ptr, 0x4);
    }
    let ptr = (VIRT_ADDR_START | 0x30008010) as *mut u32;
    unsafe {
        write_volatile(ptr, 0x40000000);
    }
    let ptr = (VIRT_ADDR_START | 0x30008004) as *mut u32;
    unsafe {
        write_volatile(ptr, 0x100006);
    }
    let ptr = (VIRT_ADDR_START | 0x3000803c) as *mut u32;
    unsafe {
        write_volatile(ptr, 0x21);
    }
    info!("nvme pci 配置完毕");
}

driver_define!("nvme", {
    // 初始化 pci
    config_pci();

    // 创建存储设备
    let device = VirtIOBlock(NvmeInterface::<DmaAllocatorImpl, IrqControllerImpl>::new(
        VIRT_ADDR_START | 0x40000000,
    ));
    let mut buffer = vec![0u8; 512];
    device.read_block(0, &mut buffer);
    // 加入设备表
    BLK_DEVICES.lock().push(Arc::new(device));
    None
});