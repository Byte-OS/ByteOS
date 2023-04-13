pub mod consts;
mod task;

use core::ffi::CStr;

pub use task::exec_with_process;

use log::warn;

use self::{
    consts::{LinuxError, SYS_EXECVE},
    task::sys_execve,
};

pub async fn syscall(call_type: usize, args: [usize; 7]) -> Result<usize, LinuxError> {
    match call_type {
        SYS_EXECVE => sys_execve(args[0] as _, args[1] as _, args[2] as _).await,
        _ => {
            warn!("unsupported syscall");
            Err(LinuxError::EPERM)
        }
    }
}

pub fn c2rust_list<T>(ptr: *mut T, valid: fn(T) -> bool) -> &'static mut [T] {
    unsafe {
        let mut len = 0;
        if !ptr.is_null() {
            loop {
                if !valid(ptr.add(len).read()) {
                    break;
                }
                len += 1;
            }
        }
        core::slice::from_raw_parts_mut(ptr, len)
    }
}

pub fn c2rust_str(ptr: *const i8) -> &'static str {
    if ptr.is_null() {
        return "";
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().unwrap()
}
