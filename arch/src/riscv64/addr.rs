use core::{
    fmt::{Debug, Display},
    ops::Add,
    slice::from_raw_parts_mut,
};

use crate::{ppn_c, PAGE_SIZE};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub(crate) usize);
impl From<PhysPage> for PhysAddr {
    fn from(value: PhysPage) -> Self {
        Self(value.0 << 12)
    }
}

impl PhysAddr {
    pub fn addr(&self) -> usize {
        self.0
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub(crate) usize);

impl From<usize> for VirtAddr {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<VirtAddr> for usize {
    fn from(value: VirtAddr) -> Self {
        value.0
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPage(pub(crate) usize);

impl From<usize> for PhysPage {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<PhysPage> for usize {
    fn from(value: PhysPage) -> Self {
        value.0
    }
}

impl Add<PhysPage> for PhysPage {
    type Output = PhysPage;

    fn add(self, rhs: PhysPage) -> Self::Output {
        PhysPage(self.0 + rhs.0)
    }
}

impl Add<usize> for PhysPage {
    type Output = PhysPage;

    fn add(self, rhs: usize) -> Self::Output {
        PhysPage(self.0 + rhs)
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtPage(pub(crate) usize);

impl PhysPage {
    #[inline]
    pub const fn new(ppn: usize) -> Self {
        Self(ppn)
    }

    #[inline]
    pub const fn from_addr(addr: usize) -> Self {
        Self(addr >> 12)
    }

    #[inline]
    pub const fn to_addr(&self) -> usize {
        self.0 << 12
    }

    #[inline]
    pub fn copy_value_from_another(&self, ppn: PhysPage) {
        unsafe {
            let src = from_raw_parts_mut(ppn_c(ppn).to_addr() as *mut u8, PAGE_SIZE);
            let dst = from_raw_parts_mut(ppn_c(*self).to_addr() as *mut u8, PAGE_SIZE);
            dst.copy_from_slice(src);
        }
    }
}

impl Add<usize> for VirtPage {
    type Output = VirtPage;

    fn add(self, rhs: usize) -> Self::Output {
        VirtPage(self.0 + rhs)
    }
}

impl PhysAddr {
    #[inline]
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }
}

impl VirtPage {
    #[inline]
    pub const fn new(vpn: usize) -> Self {
        Self(vpn)
    }

    #[inline]
    pub const fn from_addr(addr: usize) -> Self {
        Self(addr >> 12)
    }
    #[inline]
    pub const fn to_addr(&self) -> usize {
        self.0 << 12
    }
}

impl VirtAddr {
    #[inline]
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }
}

impl Display for PhysPage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:#x}", self.0))
    }
}

impl Display for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:#x}", self.0))
    }
}

impl Display for VirtPage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:#x}", self.0))
    }
}

impl Display for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:#x}", self.0))
    }
}

impl Debug for PhysPage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:#x}", self.0))
    }
}

impl Debug for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:#x}", self.0))
    }
}

impl Debug for VirtPage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:#x}", self.0))
    }
}

impl Debug for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:#x}", self.0))
    }
}
