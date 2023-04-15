pub mod consts;
mod fd;
mod task;

use core::ffi::CStr;
pub use task::exec_with_process;

use log::warn;

use self::{
    consts::{LinuxError, SYS_DUP, SYS_DUP3, SYS_EXECVE, SYS_EXIT, SYS_WRITE, SYS_OPENAT, SYS_CLOSE, SYS_READ},
    fd::{sys_dup, sys_dup3, sys_write, sys_openat, sys_close, sys_read},
    task::{sys_execve, sys_exit},
};

pub async fn syscall(call_type: usize, args: [usize; 7]) -> Result<usize, LinuxError> {
    match call_type {
        SYS_OPENAT => sys_openat(args[0] as _, args[1] as _, args[2] as _, args[3] as _).await,
        SYS_DUP => sys_dup(args[0]).await,
        SYS_DUP3 => sys_dup3(args[0], args[1]).await,
        SYS_CLOSE => sys_close(args[0] as _).await,
        SYS_READ => sys_read(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_WRITE => sys_write(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_EXECVE => sys_execve(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_EXIT => sys_exit(args[0] as _),
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

pub fn c2rust_buffer<T>(ptr: *mut T, count: usize) -> &'static mut [T] {
    unsafe { core::slice::from_raw_parts_mut(ptr, count) }
}

pub fn c2rust_str(ptr: *const i8) -> &'static str {
    if ptr.is_null() {
        return "";
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().unwrap()
}
