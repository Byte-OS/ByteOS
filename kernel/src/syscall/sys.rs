use log::debug;

use crate::syscall::{c2rust_ref, consts::UTSname};

use super::consts::LinuxError;

pub async fn sys_uname(uts_ptr: usize) -> Result<usize, LinuxError> {
    debug!("sys_uname @ uts_ptr: {:#x}", uts_ptr);
    let uts = c2rust_ref(uts_ptr as *mut UTSname);

    let sys_name = b"ByteOS";
    let sys_nodename = b"ByteOS";
    let sys_release = b"release";
    let sys_version = b"alpha 1.1";
    let sys_machine = b"riscv qemu";
    let sys_domain = b"";

    // for linux app compatible
    // let sys_name = b"Linux";
    // let sys_nodename = b"debian";
    // let sys_release = b"5.10.0-7-riscv64";
    // let sys_version = b"#1 SMP Debian 5.10.40-1 (2021-05-28)";
    // let sys_machine = b"riscv qemu";
    // let sys_domain = b"";

    uts.sysname[..sys_name.len()].copy_from_slice(sys_name);
    uts.nodename[..sys_nodename.len()].copy_from_slice(sys_nodename);
    uts.release[..sys_release.len()].copy_from_slice(sys_release);
    uts.version[..sys_version.len()].copy_from_slice(sys_version);
    uts.machine[..sys_machine.len()].copy_from_slice(sys_machine);
    uts.domainname[..sys_domain.len()].copy_from_slice(sys_domain);
    Ok(0)
}
