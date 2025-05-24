//! This file contains constants used in the libc crate.
//!
//!
/// 表示更新时间为当前时间（用于 utimensat 等系统调用）
pub const UTIME_NOW: usize = 0x3fffffff;

/// 表示不修改对应的时间字段（用于 utimensat 等系统调用）
pub const UTIME_OMIT: usize = 0x3ffffffe;
