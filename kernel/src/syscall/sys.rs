use executor::current_task;
use log::{debug, warn};

use crate::syscall::consts::{Rlimit, UTSname};
use crate::syscall::func::c2rust_ref;

use super::consts::LinuxError;

pub async fn sys_uname(uts_ptr: usize) -> Result<usize, LinuxError> {
    debug!("sys_uname @ uts_ptr: {:#x}", uts_ptr);
    let uts = c2rust_ref(uts_ptr as *mut UTSname);

    // let sys_name = b"ByteOS";
    // let sys_nodename = b"ByteOS";
    // let sys_release = b"release";
    // let sys_version = b"alpha 1.1";
    // let sys_machine = b"riscv qemu";
    // let sys_domain = b"";

    // for linux app compatible
    let sys_name = b"Linux";
    let sys_nodename = b"debian";
    let sys_release = b"5.10.0-7-riscv64";
    let sys_version = b"#1 SMP Debian 5.10.40-1 (2021-05-28)";
    let sys_machine = b"riscv qemu";
    let sys_domain = b"";

    uts.sysname[..sys_name.len()].copy_from_slice(sys_name);
    uts.nodename[..sys_nodename.len()].copy_from_slice(sys_nodename);
    uts.release[..sys_release.len()].copy_from_slice(sys_release);
    uts.version[..sys_version.len()].copy_from_slice(sys_version);
    uts.machine[..sys_machine.len()].copy_from_slice(sys_machine);
    uts.domainname[..sys_domain.len()].copy_from_slice(sys_domain);
    Ok(0)
}

/// TODO: FINISH sys_getrlimit
pub async fn sys_prlimit64(
    pid: usize,
    resource: usize,
    new_limit: usize,
    old_limit: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_getrlimit @ pid: {}, resource: {}, new_limit: {:#x}, old_limit: {:#x}",
        pid, resource, new_limit, old_limit
    );
    let user_task = current_task().as_user_task().unwrap();
    match resource {
        7 => {
            if new_limit != 0 {
                let rlimit = c2rust_ref(new_limit as *mut Rlimit);
                user_task.inner_map(|mut x| {
                    x.rlimits[7] = rlimit.max;
                })
            }
            if old_limit != 0 {
                let rlimit = c2rust_ref(old_limit as *mut Rlimit);
                rlimit.max = user_task.inner_map(|inner| inner.rlimits[7]);
                rlimit.curr = rlimit.max;
            }
        }
        _ => {
            warn!("need to finish prlimit64: resource {}", resource)
        }
    }
    Ok(0)
}

pub async fn sys_geteuid() -> Result<usize, LinuxError> {
    Ok(0)
}

pub async fn sys_getegid() -> Result<usize, LinuxError> {
    Ok(0)
}

pub async fn sys_getgid() -> Result<usize, LinuxError> {
    Ok(0)
}

pub async fn sys_getuid() -> Result<usize, LinuxError> {
    Ok(0)
}

pub async fn sys_getpgid() -> Result<usize, LinuxError> {
    Ok(0)
}

pub async fn sys_setpgid(pid: usize, pgid: usize) -> Result<usize, LinuxError> {
    warn!("set_pgid @ pid: {}, pgid: {}", pid, pgid);
    Ok(0)
}
