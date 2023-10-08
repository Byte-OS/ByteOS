use executor::current_task;
use log::{debug, warn};

use crate::syscall::consts::{Rlimit, UTSname};

use super::consts::{LinuxError, UserRef};

pub async fn sys_uname(uts_ptr: UserRef<UTSname>) -> Result<usize, LinuxError> {
    debug!("sys_uname @ uts_ptr: {}", uts_ptr);
    let uts = uts_ptr.get_mut();
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
    new_limit: UserRef<Rlimit>,
    old_limit: UserRef<Rlimit>,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_getrlimit @ pid: {}, resource: {}, new_limit: {}, old_limit: {}",
        pid, resource, new_limit, old_limit
    );
    let user_task = current_task().as_user_task().unwrap();
    match resource {
        7 => {
            if new_limit.is_valid() {
                let rlimit = new_limit.get_mut();
                user_task.inner_map(|x| {
                    x.rlimits[7] = rlimit.max;
                })
            }
            if old_limit.is_valid() {
                let rlimit = old_limit.get_mut();
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

pub async fn sys_setpgid(_pid: usize, _pgid: usize) -> Result<usize, LinuxError> {
    Ok(0)
}

pub async fn sys_klogctl(
    log_type: usize,
    buf: UserRef<u8>,
    len: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_klogctl @ log_type: {:?} buf: {:?} len: {:?}",
        log_type, buf, len
    );
    if buf.is_valid() {
        let path = buf.get_cstr().expect("can't log file to control");
        println!("{}", path);
    }
    Ok(0)
}

pub async fn sys_info(meminfo: UserRef<u8>) -> Result<usize, LinuxError> {
    debug!("sys_info: {}", meminfo);
    if meminfo.is_valid() {
        *meminfo.get_mut() = 3;
    }
    Ok(0)
}

pub async fn sys_sched_getparam(pid: usize, param: usize) -> Result<usize, LinuxError> {
    debug!("sys_sched_getparam @ pid: {} param: {}", pid, param);

    Ok(0)
}

pub async fn sys_sched_setscheduler(
    pid: usize,
    _policy: usize,
    param: usize,
) -> Result<usize, LinuxError> {
    debug!("sys_sched_setscheduler @ pid: {} param: {}", pid, param);

    Ok(0)
}

pub async fn sys_getrandom(
    buf: UserRef<u8>,
    buf_len: usize,
    flags: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_getrandom @ buf: {}, buf_len: {:#x}, flags: {:#x}",
        buf, buf_len, flags
    );
    let buf = buf.slice_mut_with_len(buf_len);
    static mut SEED: u64 = 0xdead_beef_cafe_babe;
    for x in buf.iter_mut() {
        unsafe {
            // from musl
            SEED = SEED.wrapping_mul(0x5851_f42d_4c95_7f2d);
            *x = (SEED >> 33) as u8;
        }
    }
    Ok(buf_len)
}
