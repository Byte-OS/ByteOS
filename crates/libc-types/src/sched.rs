//! This module provides the `libc` types for SCHED (scheduling).
//!
//! MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/sched.h>

bitflags! {
    /// 克隆标志，用于 `clone(2)` 或 `clone3(2)` 系统调用。
    ///
    /// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/sched.h#L50>
    #[derive(Debug)]
    pub struct CloneFlags: usize {
        /// 指定发送给子进程的信号（低 8 位），如 SIGCHLD
        const CSIGNAL                = 0x000000ff;
        /// 使用新的 time 命名空间（Linux 5.6+）
        const CLONE_NEWTIME          = 0x00000080;
        /// 与父进程共享内存空间（即使用相同地址空间）
        const CLONE_VM               = 0x00000100;
        /// 与父进程共享文件系统信息（当前目录、root 等）
        const CLONE_FS               = 0x00000200;
        /// 与父进程共享打开的文件描述符
        const CLONE_FILES            = 0x00000400;
        /// 与父进程共享信号处理函数
        const CLONE_SIGHAND          = 0x00000800;
        /// 将 PIDFD 文件描述符写入 clone 参数中指定的位置
        const CLONE_PIDFD            = 0x00001000;
        /// 被调试器使用，子进程会被 trace（如 ptrace）
        const CLONE_PTRACE           = 0x00002000;
        /// 以 vfork 语义启动子进程，阻塞父进程直到 exec/exit
        const CLONE_VFORK            = 0x00004000;
        /// 设置父进程为新进程的 parent，而不是调用进程
        const CLONE_PARENT           = 0x00008000;
        /// 与父进程成为线程（共享 signal、VM、文件等）
        const CLONE_THREAD           = 0x00010000;
        /// 使用新的挂载命名空间（mount namespace）
        const CLONE_NEWNS            = 0x00020000;
        /// 与父进程共享 System V 信号量
        const CLONE_SYSVSEM          = 0x00040000;
        /// 设置 TLS（线程局部存储）指针
        const CLONE_SETTLS           = 0x00080000;
        /// 在指定地址写入子进程的 TID（parent 设置）
        const CLONE_PARENT_SETTID    = 0x00100000;
        /// 进程退出时自动清除 TID（通常用于 futex 唤醒）
        const CLONE_CHILD_CLEARTID   = 0x00200000;
        /// 被废弃，曾用于标记 detached 线程
        const CLONE_DETACHED         = 0x00400000;
        /// 禁用子进程被 trace
        const CLONE_UNTRACED         = 0x00800000;
        /// 在指定地址写入子进程的 TID（child 设置）
        const CLONE_CHILD_SETTID     = 0x01000000;
        /// 使用新的 cgroup 命名空间（隔离控制组）
        const CLONE_NEWCGROUP        = 0x02000000;
        /// 使用新的 UTS 命名空间（隔离主机名/域名）
        const CLONE_NEWUTS           = 0x04000000;
        /// 使用新的 IPC 命名空间（隔离 System V IPC）
        const CLONE_NEWIPC           = 0x08000000;
        /// 使用新的用户命名空间（user namespace）
        const CLONE_NEWUSER          = 0x10000000;
        /// 使用新的 PID 命名空间（隔离进程号）
        const CLONE_NEWPID           = 0x20000000;
        /// 使用新的网络命名空间（network namespace）
        const CLONE_NEWNET           = 0x40000000;
        /// 启用 I/O 上下文的共享（Linux 2.6.25+）
        const CLONE_IO               = 0x80000000;
    }
}
