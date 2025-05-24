//! This module provides the `libc` types for Other unclassified types.
//!
//!

use num_enum::TryFromPrimitive;

/// Architecture-specific command for the `arch_prctl` syscall.
#[repr(usize)]
#[derive(Debug, Clone, TryFromPrimitive)]
pub enum ArchPrctlCmd {
    /// Set Per-CPU base
    SetGS = 0x1001,
    /// Set Thread Local Storage (TLS) base
    SetFS = 0x1002,
    /// Get Thread Local Storage (TLS) base
    GetFS = 0x1003,
    /// Get Per-CPU base
    GetGS = 0x1004,
}
