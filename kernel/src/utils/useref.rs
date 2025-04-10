use core::{
    fmt::{Debug, Display},
    marker::PhantomData,
};

use polyhal::VirtAddr;

#[derive(Clone, Copy)]
pub struct UserRef<T> {
    addr: VirtAddr,
    r#type: PhantomData<T>,
}

impl<T> From<usize> for UserRef<T> {
    fn from(value: usize) -> Self {
        Self {
            addr: value.into(),
            r#type: PhantomData,
        }
    }
}

impl<T> From<VirtAddr> for UserRef<T> {
    fn from(value: VirtAddr) -> Self {
        Self {
            addr: value,
            r#type: PhantomData,
        }
    }
}

impl<T> Into<usize> for UserRef<T> {
    fn into(self) -> usize {
        self.addr.raw()
    }
}

impl<T> UserRef<T> {
    #[inline]
    pub fn addr(&self) -> usize {
        self.addr.raw()
    }
    #[inline]
    pub fn get_ref(&self) -> &'static T {
        self.addr.get_ref::<T>()
    }

    #[inline]
    pub fn get_mut(&self) -> &'static mut T {
        self.addr.get_mut_ref::<T>()
    }

    #[inline]
    pub fn slice_mut_with_len(&self, len: usize) -> &'static mut [T] {
        self.addr.slice_mut_with_len(len)
    }

    #[inline]
    pub fn slice_until_valid(&self, is_valid: fn(T) -> bool) -> &'static mut [T] {
        if self.addr.raw() == 0 {
            return &mut [];
        }
        self.addr.slice_until(is_valid)
    }

    #[inline]
    pub fn get_cstr(&self) -> Result<&str, core::str::Utf8Error> {
        self.addr.get_cstr().to_str()
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.addr.raw() != 0
    }
}

impl<T> Display for UserRef<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "{}({:#x})",
            core::any::type_name::<T>(),
            self.addr.raw()
        ))
    }
}

impl<T> Debug for UserRef<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "{}({:#x})",
            core::any::type_name::<T>(),
            self.addr.raw()
        ))
    }
}
