use core::{
    ffi::CStr,
    fmt::{Debug, Display},
    mem::size_of,
    ops::Add,
};

use crate::{PAGE_SIZE, VIRT_ADDR_START};

#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub(crate) usize);
impl From<PhysPage> for PhysAddr {
    fn from(value: PhysPage) -> Self {
        Self(value.0 << 12)
    }
}

impl PhysAddr {
    #[inline]
    pub fn addr(&self) -> usize {
        self.0
    }

    #[inline]
    pub fn get_ptr<T>(&self) -> *const T {
        (self.0 | VIRT_ADDR_START) as *const T
    }

    #[inline]
    pub const fn get_mut_ptr<T>(&self) -> *mut T {
        (self.0 | VIRT_ADDR_START) as *mut T
    }

    #[inline]
    pub fn slice_with_len<T>(&self, len: usize) -> &'static [T] {
        unsafe { core::slice::from_raw_parts(self.get_ptr(), len) }
    }

    #[inline]
    pub fn slice_mut_with_len<T>(&self, len: usize) -> &'static mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.get_mut_ptr(), len) }
    }

    #[inline]
    pub fn get_cstr(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.get_ptr::<i8>()) }
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

impl VirtAddr {
    #[inline]
    pub fn addr(&self) -> usize {
        self.0
    }

    #[inline]
    pub fn get_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    #[inline]
    pub fn get_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    #[inline]
    pub fn get_ref<T>(&self) -> &'static T {
        unsafe { &*(self.0 as *const T) }
    }

    #[inline]
    pub fn get_mut_ref<T>(&self) -> &'static mut T {
        unsafe { &mut *(self.0 as *mut T) }
    }

    #[inline]
    pub fn slice_with_len<T>(&self, len: usize) -> &'static [T] {
        unsafe { core::slice::from_raw_parts(self.get_ptr(), len) }
    }

    #[inline]
    pub fn slice_mut_with_len<T>(&self, len: usize) -> &'static mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.get_mut_ptr(), len) }
    }

    #[inline]
    pub fn slice_until<T>(&self, is_valid: fn(T) -> bool) -> &'static mut [T] {
        let ptr = self.addr() as *mut T;
        unsafe {
            let mut len = 0;
            if !ptr.is_null() {
                loop {
                    if !is_valid(ptr.add(len).read()) {
                        break;
                    }
                    len += 1;
                }
            }
            core::slice::from_raw_parts_mut(ptr, len)
        }
    }

    #[inline]
    pub fn get_cstr(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.get_ptr::<i8>()) }
    }

    #[inline]
    pub fn floor(&self) -> Self {
        Self(self.0 / PAGE_SIZE * PAGE_SIZE)
    }

    #[inline]
    pub fn ceil(&self) -> Self {
        Self((self.0 + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE)
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPage(pub(crate) usize);

impl From<usize> for PhysPage {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<PhysAddr> for PhysPage {
    fn from(value: PhysAddr) -> Self {
        Self(value.0 >> 12)
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
impl From<VirtAddr> for VirtPage {
    fn from(value: VirtAddr) -> Self {
        Self(value.0 >> 12)
    }
}
impl From<usize> for VirtPage {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

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
    pub const fn get_buffer(&self) -> &'static mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut((self.0 << 12 | VIRT_ADDR_START) as *mut u8, PAGE_SIZE)
        }
    }

    #[inline]
    pub fn copy_value_from_another(&self, ppn: PhysPage) {
        self.get_buffer().copy_from_slice(&ppn.get_buffer());
        #[cfg(c906)]
        unsafe {
            asm!(".long 0x0010000b"); // dcache.all
            asm!(".long 0x01b0000b"); // sync.is
        }
    }

    #[inline]
    pub fn drop_clear(&self) {
        // self.get_buffer().fill(0);
        unsafe {
            core::slice::from_raw_parts_mut(
                (self.0 << 12 | VIRT_ADDR_START) as *mut usize,
                PAGE_SIZE / size_of::<usize>(),
            )
            .fill(0);
        }
        #[cfg(c906)]
        unsafe {
            asm!(".long 0x0010000b"); // dcache.all
            asm!(".long 0x01b0000b"); // sync.is
        }
    }

    #[inline]
    pub fn as_num(&self) -> usize {
        self.0
    }
}

impl Add<usize> for VirtPage {
    type Output = VirtPage;

    fn add(self, rhs: usize) -> Self::Output {
        VirtPage(self.0 + rhs)
    }
}

impl From<VirtPage> for VirtAddr {
    fn from(value: VirtPage) -> Self {
        Self(value.to_addr())
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
