//! This module provides the `libc` types for MMAN (memory management).
//!
//! MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/sys/mman.h>

bitflags! {
    /// MAP 标志位（用于 mmap 等内存映射操作）
    ///
    /// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/sys/mman.h#L22>
    #[derive(Debug)]
    pub struct MapFlags: u32 {
        /// 共享映射，写入会直接影响文件内容
        const SHARED          = 0x01;
        /// 私有映射，写入会产生写时复制（Copy-on-Write）
        const PRIVATE         = 0x02;
        /// 验证共享映射（共享或私有），与 MAP_SHARED_VALIDATE 相关
        const SHARED_VALIDATE = 0x03;
        /// 映射类型掩码（用于屏蔽高位判断映射类型）
        const TYPE            = 0x0f;
        /// 使用固定地址映射，映射必须在指定地址
        const FIXED           = 0x10;
        /// 匿名映射，不与任何文件关联（内容初始化为 0）
        const ANONYMOUS       = 0x20;
        /// 不保留交换空间（swap）
        const NORESERVE       = 0x4000;
        /// 堆栈向下增长区域（如线程栈）
        const GROWSDOWN       = 0x0100;
        /// 拒绝写操作（通常用于文件系统写保护）
        const DENYWRITE       = 0x0800;
        /// 映射可执行代码（允许执行权限）
        const EXECUTABLE      = 0x1000;
        /// 映射锁定在内存中，避免换出
        const LOCKED          = 0x2000;
        /// 预先加载页面（降低缺页中断）
        const POPULATE        = 0x8000;
        /// 非阻塞映射
        const NONBLOCK        = 0x10000;
        /// 映射用作线程栈
        const STACK           = 0x20000;
        /// 使用大页（HugeTLB）
        const HUGETLB         = 0x40000;
        /// 同步映射（同步内存访问）
        const SYNC            = 0x80000;
        /// 固定映射，但不覆盖已有映射
        const FIXED_NOREPLACE = 0x100000;
        /// 文件映射（默认标志）
        const FILE            = 0;
    }


    #[derive(Debug, Clone, Copy)]
    /// 内存映射保护标志（mmap 的 prot 参数）
    /// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/sys/mman.h#L57>
    pub struct MmapProt: u32 {
        /// 可读权限，映射区域可被读取
        const READ = bit!(0);
        /// 可写权限，映射区域可被写入
        const WRITE = bit!(1);
        /// 可执行权限，映射区域允许执行代码
        const EXEC = bit!(2);
    }

    #[derive(Debug)]
    /// msync 同步标志，用于控制 msync 行为
    ///
    /// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/sys/mman.h#L64C9-L64C17>
    pub struct MSyncFlags: u32 {
        /// 异步同步（异步刷新内存映射区域到存储设备）
        const ASYNC = 1 << 0;
        /// 使其他缓存失效（使缓存区域无效）
        const INVALIDATE = 1 << 1;
        /// 同步同步（阻塞直到数据完全写入存储设备）
        const SYNC = 1 << 2;
    }


}
