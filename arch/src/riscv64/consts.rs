pub const VIRT_ADDR_START: usize = 0xffff_ffc0_0000_0000;
pub const USER_ADDR_MAX: usize = 0xbf_ffff_ffff;
pub const PAGE_SIZE: usize = 4096;
pub const PAGE_ITEM_COUNT: usize = 512;
pub const SIG_RETURN_ADDR: usize = 0xFFFF_FFC1_0000_0000;

/// Every core has a unique area of memory.
/// Just using pagetable to map multi core area.
/// Area size: 0x100_0000 (16MBytes)
///
/// First Area is 0xFFFF_FFC2_0000_0000
/// Next Area is 0xFFFF_FFC2_0100_0000
/// Others Same as This, so it will support 16 * 16 = 256 cores (Only auxiliary Harts).
pub const MULTI_CORE_AREA: usize = 0xFFFF_FFC2_0000_0000;
pub const MULTI_CORE_AREA_SIZE: usize = 0x100_0000;
