#![allow(unused)]
pub mod elf {
    /// 结束标志
    pub const AT_NULL: usize = 0;
    /// 被忽略的项
    pub const AT_IGNORE: usize = 1;
    /// 文件描述符（指向可执行文件），在内核加载器中用
    pub const AT_EXECFD: usize = 2;
    /// 程序头表（Program Header Table）在内存中的地址
    pub const AT_PHDR: usize = 3;
    /// 每个程序头的大小（以字节为单位）
    pub const AT_PHENT: usize = 4;
    /// 程序头的数量
    pub const AT_PHNUM: usize = 5;
    /// 页大小（单位字节），如 4096
    pub const AT_PAGESZ: usize = 6;
    /// 动态链接器的基址（即 ld.so 的加载地址）
    pub const AT_BASE: usize = 7;
    /// 运行时标志，通常为 0
    pub const AT_FLAGS: usize = 8;
    /// 程序入口点（Entry Point）
    pub const AT_ENTRY: usize = 9;
    /// 如果是非 ELF 二进制（a.out 格式），为 1，否则为 0
    pub const AT_NOTELF: usize = 10;
    /// 实际用户 ID（UID）
    pub const AT_UID: usize = 11;
    /// 有效用户 ID（EUID）
    pub const AT_EUID: usize = 12;
    /// 实际组 ID（GID）
    pub const AT_GID: usize = 13;
    /// 有效组 ID（EGID）
    pub const AT_EGID: usize = 14;
    /// CPU 平台名称的指针（如 "x86_64"）
    pub const AT_PLATFORM: usize = 15;
    /// 硬件能力位（bitmask），如 SSE/AVX 支持
    pub const AT_HWCAP: usize = 16;
    /// 每秒的时钟滴答数（用于 `times()` 等函数）
    pub const AT_CLKTCK: usize = 17;

    // 以下是部分平台特定或旧版本字段
    /// x86 FPU 控制字（FPUCW）
    pub const AT_FPUCW: usize = 18;
    /// D-cache（数据缓存）大小
    pub const AT_DCACHEBSIZE: usize = 19;
    /// I-cache（指令缓存）大小
    pub const AT_ICACHEBSIZE: usize = 20;
    /// 通用缓存大小
    pub const AT_UCACHEBSIZE: usize = 21;

    /// PowerPC 平台专用，被忽略
    pub const AT_IGNOREPPC: usize = 22;

    /// 是否是安全模式（非 suid/guid），0 = 否，1 = 是
    pub const AT_SECURE: usize = 23;
    /// 基础平台名称的指针（字符串）
    pub const AT_BASE_PLATFORM: usize = 24;
    /// 指向随机数种子（stack 上的 16 字节随机值）
    pub const AT_RANDOM: usize = 25;
    /// 第二组 HWCAP（arm64/aarch64）
    pub const AT_HWCAP2: usize = 26;

    // 27~30 保留或未使用

    /// 命令行中可执行文件路径的地址（如 "/bin/ls"）
    pub const AT_EXECFN: usize = 31;
    /// 指向 vsyscall 区域的函数地址（如 `gettimeofday()`）
    pub const AT_SYSINFO: usize = 32;
    /// 指向 VDSO ELF 映射的起始地址
    pub const AT_SYSINFO_EHDR: usize = 33;
}
