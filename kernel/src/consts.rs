use fs::pathbuf::PathBuf;

/// 默认的用户程序入口函数
pub const USER_WORK_DIR: PathBuf = PathBuf::new();

/// 用户态动态链接用户程序的偏移
pub const USER_DYN_ADDR: usize = 0x20000000;

/// 用户态栈顶
pub const USER_STACK_TOP: usize = 0x8000_0000;

/// 用户栈初始大小
pub const USER_STACK_INIT_SIZE: usize = 0x20000;
