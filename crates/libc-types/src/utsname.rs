//! This module provides the `libc` types for UTSNAME (Unix Time Sharing Name).
//!
//!

/// 系统信息结构体（对应 `struct utsname`），用于表示内核和主机相关信息
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/sys/utsname.h#L9>
pub struct UTSname {
    /// 操作系统名称，例如 "Linux"
    pub sysname: [u8; 65],
    /// 主机名称，例如 "my-hostname"
    pub nodename: [u8; 65],
    /// 内核发行版本，例如 "5.15.0"
    pub release: [u8; 65],
    /// 内核版本信息，例如 "#1 SMP PREEMPT_DYNAMIC ..."
    pub version: [u8; 65],
    /// 机器架构，例如 "x86_64"
    pub machine: [u8; 65],
    /// 域名，例如 "(none)" 或 "example.com"
    pub domainname: [u8; 65],
}
