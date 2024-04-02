use core::ops::{Index, IndexMut};

use crate::{KContextArgs, PageTable};

/// Kernel Context
///
/// Kernel Context is used to switch context between kernel task.
#[derive(Debug)]
#[repr(C)]
pub struct KContext {
    /// Kernel Stack Pointer
    ksp: usize,
    /// Kernel Thread Pointer
    ktp: usize,
    // Callee saved registers, x19..x29
    _regs: [usize; 11],
    /// Kernel Program Counter, Will return to this address.
    kpc: usize,
}

impl KContext {
    /// Create a new blank Kernel Context.
    pub fn blank() -> Self {
        Self {
            ksp: 0,
            ktp: 0,
            _regs: [0; 11],
            kpc: 0,
        }
    }
}

/// Indexing operations for KContext
///
/// Using it just like the Vector.
///
/// #[derive(Debug)]
/// pub enum KContextArgs {
///     /// Kernel Stack Pointer
///     KSP,
///     /// Kernel Thread Pointer
///     KTP,
///     /// Kernel Program Counter
///     KPC
/// }
///
/// etc. Get reg of the kernel stack:
///
/// let ksp = KContext[KContextArgs::KSP]
/// let kpc = KContext[KContextArgs::KPC]
/// let ktp = KContext[KContextArgs::KTP]
///
impl Index<KContextArgs> for KContext {
    type Output = usize;

    fn index(&self, index: KContextArgs) -> &Self::Output {
        match index {
            KContextArgs::KSP => &self.ksp,
            KContextArgs::KTP => &self.ktp,
            KContextArgs::KPC => &self.kpc,
        }
    }
}

/// Indexing Mutable operations for KContext
///
/// Using it just like the Vector.
///
/// etc. Change the value of the kernel Context using IndexMut
///
/// KContext[KContextArgs::KSP] = ksp;
/// KContext[KContextArgs::KPC] = kpc;
/// KContext[KContextArgs::KTP] = ktp;
///
impl IndexMut<KContextArgs> for KContext {
    fn index_mut(&mut self, index: KContextArgs) -> &mut Self::Output {
        match index {
            KContextArgs::KSP => &mut self.ksp,
            KContextArgs::KTP => &mut self.ktp,
            KContextArgs::KPC => &mut self.kpc,
        }
    }
}

/// Context Switch
///
/// Save the context of current task and switch to new task.
#[naked]
pub unsafe extern "C" fn context_switch(from: *mut KContext, to: *const KContext) {
    core::arch::asm!(
        // Save Kernel Context.
        "
            mrs     x3,  tpidr_el1
            mov     x4,  sp
            stp     x4,  x3,  [x0]
            stp     x19, x20, [x0,  2 * 8]
            stp     x21, x22, [x0,  4 * 8]
            stp     x23, x24, [x0,  6 * 8]
            stp     x25, x26, [x0,  8 * 8]
            stp     x27, x28, [x0, 10 * 8]
            stp     x27, x28, [x0, 10 * 8]
            stp     x29, x30, [x0, 12 * 8]
        ",
        // Restore Kernel Context.
        "
            ldp     x4,  x3,  [x1]
            ldp     x19, x20, [x1,  2 * 8]
            ldp     x21, x22, [x1,  4 * 8]
            ldp     x23, x24, [x1,  6 * 8]
            ldp     x25, x26, [x1,  8 * 8]
            ldp     x27, x28, [x1, 10 * 8]
            ldp     x27, x28, [x1, 10 * 8]
            ldp     x29, x30, [x1, 12 * 8]
            msr     tpidr_el1, x3
            mov     sp, x4
            ret
        ",
        options(noreturn)
    )
}

/// Context Switch With Page Table
///
/// Save the context of current task and switch to new task.
#[naked]
pub unsafe extern "C" fn context_switch_pt(
    from: *mut KContext,
    to: *const KContext,
    pt_token: PageTable,
) {
    core::arch::asm!(
        // Save Kernel Context.
        "
            mrs     x3, tpidr_el1
            mov     x4, sp
            stp     x4,  x3,  [x0]
            stp     x19, x20, [x0,  2 * 8]
            stp     x21, x22, [x0,  4 * 8]
            stp     x23, x24, [x0,  6 * 8]
            stp     x25, x26, [x0,  8 * 8]
            stp     x27, x28, [x0, 10 * 8]
            stp     x27, x28, [x0, 10 * 8]
            stp     x29, x30, [x0, 12 * 8]
        ",
        // Switch to new page table.
        "
            msr     ttbr0_el1, x2
            tlbi vmalle1 
            dsb sy
            isb
        ",
        // Restore Kernel Context.
        "
            ldp     x4,  x3,  [x1]
            ldp     x19, x20, [x1,  2 * 8]
            ldp     x21, x22, [x1,  4 * 8]
            ldp     x23, x24, [x1,  6 * 8]
            ldp     x25, x26, [x1,  8 * 8]
            ldp     x27, x28, [x1, 10 * 8]
            ldp     x27, x28, [x1, 10 * 8]
            ldp     x29, x30, [x1, 12 * 8]
            msr     tpidr_el1, x3
            mov     sp, x4
            ret
        ",
        options(noreturn)
    )
}

/// Read thread pointer currently.
#[naked]
pub extern "C" fn read_current_tp() -> usize {
    unsafe {
        core::arch::asm!(
            "
                mrs      x0, tpidr_el1
                ret
            ",
            options(noreturn)
        )
    }
}
