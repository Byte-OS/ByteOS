//! This module provides the `libc` types for ELF (Executable and Linkable Format).

/// ELF auxiliary vector (auxv) entry type
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/elf.h#L1001>
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuxType {
    /// 结束标志
    Null = 0,
    /// 被忽略的项
    Ignore = 1,
    /// 文件描述符（指向可执行文件），在内核加载器中用
    ExecFd = 2,
    /// 程序头表（Program Header Table）在内存中的地址
    Phdr = 3,
    /// 每个程序头的大小（以字节为单位）
    Phent = 4,
    /// 程序头的数量
    Phnum = 5,
    /// 页大小（单位字节），如 4096
    PageSize = 6,
    /// 动态链接器的基址（即 ld.so 的加载地址）
    Base = 7,
    /// 运行时标志，通常为 0
    Flags = 8,
    /// 程序入口点（Entry Point）
    Entry = 9,
    /// 如果是非 ELF 二进制（a.out 格式），为 1，否则为 0
    NotElf = 10,
    /// 实际用户 ID（UID）
    UID = 11,
    /// 有效用户 ID（EUID）
    EUID = 12,
    /// 实际组 ID（GID）
    GID = 13,
    /// 有效组 ID（EGID）
    EGID = 14,
    /// CPU 平台名称的指针（如 "x86_64"）
    Platform = 15,
    /// 硬件能力位（bitmask），如 SSE/AVX 支持
    HwCap = 16,
    /// 每秒的时钟滴答数（用于 `times()` 等函数）
    ClkTck = 17,
    /// x86 FPU 控制字（FPUCW）
    FpuCw = 18,
    /// D-cache（数据缓存）大小
    DCacheBSize = 19,
    /// I-cache（指令缓存）大小
    ICacheBSize = 20,
    /// 通用缓存大小
    UCacheBSize = 21,
    /// PowerPC 平台专用，被忽略
    IgnorePPC = 22,
    /// 是否是安全模式（非 suid/guid），0 = 否，1 = 是
    Secure = 23,
    /// 基础平台名称的指针（字符串）
    BasePlatform = 24,
    /// 指向随机数种子（stack 上的 16 字节随机值）
    Random = 25,
    /// 第二组 HWCAP（arm64/aarch64）
    HwCap2 = 26,
    /// 命令行中可执行文件路径的地址（如 "/bin/ls"）
    ExecFn = 31,
    /// 指向 vsyscall 区域的函数地址（如 `gettimeofday()`）
    SysInfo = 32,
    /// 指向 VDSO ELF 映射的起始地址
    SysInfoEhdr = 33,
}
