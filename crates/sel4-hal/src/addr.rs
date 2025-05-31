#![no_std]

use core::{
    ffi::{c_char, CStr},
    ops::Add,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct PAddr(usize);

impl PAddr {
    pub const fn new(v: usize) -> Self {
        Self(v)
    }

    pub const fn raw(&self) -> usize {
        self.0
    }

    pub fn clear_len(&self, len: usize) {
        todo!()
    }

    pub fn slice_with_len(&self, len: usize) -> &'static [u8] {
        todo!()
    }

    pub fn slice_mut_with_len(&self, len: usize) -> &'static mut [u8] {
        todo!()
    }
}

impl Add<usize> for PAddr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VAddr(usize);

impl VAddr {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn raw(&self) -> usize {
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
        todo!()
        // unsafe { &*(self.0 as *const T) }
    }

    #[inline]
    pub fn get_mut_ref<T>(&self) -> &'static mut T {
        todo!()
        // unsafe { &mut *(self.0 as *mut T) }
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
        let ptr = self.raw() as *mut T;
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
        unsafe { CStr::from_ptr(self.get_ptr::<c_char>()) }
    }
}

pub const PAGE_SIZE: usize = 4096;

#[macro_export]
macro_rules! pa {
    ($addr:expr) => {
        $crate::PAddr::new($addr)
    };
}

#[macro_export]
macro_rules! va {
    ($addr:expr) => {
        $crate::VAddr::new($addr)
    };
}
