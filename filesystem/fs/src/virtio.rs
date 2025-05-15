use alloc::{collections::BTreeMap, str, sync::Arc};
use common::services::root::translate_addr;
use config::{DMA_ADDR_START, VIRTIO_MMIO_BLK_VIRT_ADDR};
use core::{
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};
use devices::{
    device::{BlkDriver, Driver},
    driver_define,
};
use spin::Mutex;
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::mmio::{MmioTransport, VirtIOHeader},
    BufferDirection, Hal, PhysAddr, PAGE_SIZE,
};

static DMA_ADDR: AtomicUsize = AtomicUsize::new(DMA_ADDR_START);
static ADDR_MAP: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());

driver_define!({ Some(Arc::new(VirtIOBlock::new())) });

pub struct VirtIOBlock(Mutex<VirtIOBlk<HalImpl, MmioTransport>>);
unsafe impl Sync for VirtIOBlock {}
unsafe impl Send for VirtIOBlock {}

impl VirtIOBlock {
    pub fn new() -> Self {
        Self(Mutex::new(
            VirtIOBlk::<HalImpl, MmioTransport>::new(unsafe {
                MmioTransport::new(
                    NonNull::new(VIRTIO_MMIO_BLK_VIRT_ADDR as *mut VirtIOHeader).unwrap(),
                )
                .unwrap()
            })
            .unwrap(),
        ))
    }
}

impl Driver for VirtIOBlock {
    fn get_id(&self) -> &str {
        "virtio-blk"
    }

    fn get_device_wrapper(self: Arc<Self>) -> devices::device::DeviceType {
        devices::device::DeviceType::BLOCK(self.clone())
    }
}

impl BlkDriver for VirtIOBlock {
    fn read_blocks(&self, block_id: usize, buf: &mut [u8]) {
        self.0.lock().read_blocks(block_id, buf).unwrap();
    }

    fn write_blocks(&self, block_id: usize, buf: &[u8]) {
        self.0.lock().write_blocks(block_id, buf).unwrap();
    }

    fn capacity(&self) -> usize {
        (self.0.lock().capacity() * 0x200) as _
    }
}

pub fn init() {}

pub fn translate_address(vaddr: usize) -> usize {
    let vp_index = vaddr / PAGE_SIZE;
    let offset = vaddr % PAGE_SIZE;

    let mut map = ADDR_MAP.lock();
    let paddr = match map.get(&vp_index) {
        Some(v) => v * PAGE_SIZE + offset,
        None => {
            let paddr = translate_addr(vaddr);

            map.insert(vp_index, paddr / PAGE_SIZE);
            paddr
        }
    };
    paddr
}

pub struct HalImpl;

unsafe impl Hal for HalImpl {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (PhysAddr, NonNull<u8>) {
        let vaddr = DMA_ADDR.load(Ordering::Acquire);
        DMA_ADDR.store(vaddr + pages * PAGE_SIZE, Ordering::Release);

        (
            translate_address(vaddr),
            NonNull::new(vaddr as *mut u8).unwrap(),
        )
    }

    unsafe fn dma_dealloc(_paddr: PhysAddr, _vaddr: NonNull<u8>, _pages: usize) -> i32 {
        0
    }

    unsafe fn mmio_phys_to_virt(_paddr: PhysAddr, _size: usize) -> NonNull<u8> {
        todo!()
    }

    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> PhysAddr {
        translate_address(buffer.as_ptr() as *const u8 as _)
    }

    unsafe fn unshare(_paddr: PhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {
        // Nothing to do, as the host already has access to all memory and we didn't copy the buffer
        // anywhere else.
    }
}
