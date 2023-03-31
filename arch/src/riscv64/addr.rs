use core::{fmt::Debug, ops::Add};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(usize);
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(usize);
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPage(usize);

impl PhysPage {
    pub const fn new(addr: usize) -> Self {
        Self(addr >> 12)
    }

    #[inline]
    pub fn from_addr(addr: usize) -> Self {
        Self(addr >> 12)
    }
}

impl Debug for PhysPage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:#x}", self.0))
    }
}

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
pub struct VirtPage(usize);
