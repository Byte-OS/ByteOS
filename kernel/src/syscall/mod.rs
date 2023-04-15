pub mod consts;
mod fd;
mod mm;
mod sys;
mod task;
mod time;

use core::ffi::CStr;
pub use task::exec_with_process;

use log::warn;

use self::{
    consts::{
        LinuxError, SYS_BRK, SYS_CHDIR, SYS_CLOSE, SYS_DUP, SYS_DUP3, SYS_EXECVE, SYS_EXIT,
        SYS_FSTAT, SYS_GETCWD, SYS_GETPID, SYS_GETTIMEOFDAY, SYS_MKDIRAT, SYS_NANOSLEEP,
        SYS_OPENAT, SYS_READ, SYS_UNAME, SYS_UNLINKAT, SYS_WRITE,
    },
    fd::{
        sys_close, sys_dup, sys_dup3, sys_fstat, sys_mkdir_at, sys_openat, sys_read, sys_unlinkat,
        sys_write,
    },
    mm::sys_brk,
    sys::sys_uname,
    task::{sys_chdir, sys_execve, sys_exit, sys_getcwd, sys_getpid},
    time::{sys_gettimeofday, sys_nanosleep},
};

pub async fn syscall(call_type: usize, args: [usize; 7]) -> Result<usize, LinuxError> {
    match call_type {
        SYS_GETCWD => sys_getcwd(args[0] as _, args[1] as _).await,
        SYS_CHDIR => sys_chdir(args[0] as _).await,
        SYS_OPENAT => sys_openat(args[0] as _, args[1] as _, args[2] as _, args[3] as _).await,
        SYS_DUP => sys_dup(args[0]).await,
        SYS_DUP3 => sys_dup3(args[0], args[1]).await,
        SYS_CLOSE => sys_close(args[0] as _).await,
        SYS_MKDIRAT => sys_mkdir_at(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_READ => sys_read(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_WRITE => sys_write(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_EXECVE => sys_execve(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_EXIT => sys_exit(args[0] as _),
        SYS_BRK => sys_brk(args[0] as _).await,
        SYS_GETPID => sys_getpid().await,
        SYS_GETTIMEOFDAY => sys_gettimeofday(args[0] as _, args[1] as _).await,
        SYS_NANOSLEEP => sys_nanosleep(args[0] as _, args[1] as _).await,
        SYS_UNAME => sys_uname(args[0] as _).await,
        SYS_UNLINKAT => sys_unlinkat(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_FSTAT => sys_fstat(args[0] as _, args[1] as _).await,
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

pub fn c2rust_ref<T>(ptr: *mut T) -> &'static mut T {
    unsafe { ptr.as_mut().unwrap() }
}
