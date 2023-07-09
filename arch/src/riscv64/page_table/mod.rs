pub mod sigtrx;
mod sv39;

use riscv::register::satp;
pub use sv39::*;

use crate::PhysAddr;

#[inline]
pub fn current_page_table() -> PageTable {
    let addr = satp::read().ppn() << 12;
    PageTable::new(PhysAddr(addr))
}
