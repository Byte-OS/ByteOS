use super::SysResult;
use crate::{user::UserTaskContainer, utils::useref::UserRef};
use libc_types::{resource::Rlimit, utsname::UTSname};
use log::{debug, warn};

impl UserTaskContainer {
    pub fn sys_uname(&self, uts_ptr: UserRef<UTSname>) -> SysResult {
        debug!("sys_uname @ uts_ptr: {}", uts_ptr);

        // for linux app compatible
        let sys_name = b"Linux";
        let sys_nodename = b"debian";
        let sys_release = b"5.10.0-7-riscv64";
        let sys_version = b"#1 SMP Debian 5.10.40-1 (2021-05-28)";
        let sys_machine = b"riscv qemu";
        let sys_domain = b"";

        uts_ptr.with_mut(|uts| {
            uts.sysname[..sys_name.len()].copy_from_slice(sys_name);
            uts.nodename[..sys_nodename.len()].copy_from_slice(sys_nodename);
            uts.release[..sys_release.len()].copy_from_slice(sys_release);
            uts.version[..sys_version.len()].copy_from_slice(sys_version);
            uts.machine[..sys_machine.len()].copy_from_slice(sys_machine);
            uts.domainname[..sys_domain.len()].copy_from_slice(sys_domain);
        });
        Ok(0)
    }

    /// TODO: FINISH sys_getrlimit
    pub fn sys_prlimit64(
        &self,
        pid: usize,
        resource: usize,
        new_limit: UserRef<Rlimit>,
        old_limit: UserRef<Rlimit>,
    ) -> SysResult {
        debug!(
            "sys_getrlimit @ pid: {}, resource: {}, new_limit: {}, old_limit: {}",
            pid, resource, new_limit, old_limit
        );
        match resource {
            7 => {
                if new_limit.is_valid() {
                    let rlimit = new_limit.read();
                    self.task.pcb.lock().rlimits[7] = rlimit.max;
                }
                if old_limit.is_valid() {
                    old_limit.with_mut(|rlimit| {
                        rlimit.max = self.task.inner_map(|inner| inner.rlimits[7]);
                        rlimit.curr = rlimit.max;
                    })
                }
            }
            _ => {
                warn!("need to finish prlimit64: resource {}", resource)
            }
        }
        Ok(0)
    }

    pub fn sys_geteuid(&self) -> SysResult {
        Ok(0)
    }

    pub fn sys_getegid(&self) -> SysResult {
        Ok(0)
    }

    pub fn sys_getgid(&self) -> SysResult {
        Ok(0)
    }

    pub fn sys_getuid(&self) -> SysResult {
        Ok(0)
    }

    pub fn sys_getpgid(&self) -> SysResult {
        Ok(0)
    }

    pub fn sys_setpgid(&self, _pid: usize, _pgid: usize) -> SysResult {
        Ok(0)
    }

    pub fn sys_klogctl(&self, log_type: usize, buf: UserRef<u8>, len: usize) -> SysResult {
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

    pub fn sys_info(&self, meminfo: UserRef<u8>) -> SysResult {
        debug!("sys_info: {}", meminfo);
        if meminfo.is_valid() {
            meminfo.write(3);
        }
        Ok(0)
    }

    pub fn sys_sched_getparam(&self, pid: usize, param: usize) -> SysResult {
        debug!("sys_sched_getparam @ pid: {} param: {}", pid, param);

        Ok(0)
    }

    pub fn sys_sched_setscheduler(&self, pid: usize, _policy: usize, param: usize) -> SysResult {
        debug!("sys_sched_setscheduler @ pid: {} param: {}", pid, param);

        Ok(0)
    }

    pub fn sys_getrandom(&self, buf: UserRef<u8>, buf_len: usize, flags: usize) -> SysResult {
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

    #[cfg(target_arch = "x86_64")]
    pub fn sys_arch_prctl(&self, code: usize, addr: usize) -> SysResult {
        use libc_types::others::ArchPrctlCmd;
        use polyhal_trap::trapframe::TrapFrameArgs;
        use syscalls::Errno;

        let arch_prctl_code = ArchPrctlCmd::try_from(code).map_err(|_| Errno::EINVAL)?;
        debug!(
            "sys_arch_prctl @ code: {:?}, addr: {:#x}",
            arch_prctl_code, addr
        );
        let cx_ref = self.task.force_cx_ref();
        match arch_prctl_code {
            ArchPrctlCmd::SetFS => cx_ref[TrapFrameArgs::TLS] = addr,
            _ => {
                error!("arch prctl: {:#x?}", arch_prctl_code);
                return Err(Errno::EPERM);
            }
        }
        Ok(0)
    }
}
