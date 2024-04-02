use core::{
    arch::asm,
    ops::{Index, IndexMut},
};

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
    /// Kernel S regs, s0 - s11, just callee-saved registers
    /// just used in the context_switch function.
    _sregs: [usize; 12],
    /// Kernel Program Counter, Will return to this address.
    kpc: usize,
}

impl KContext {
    /// Create a new blank Kernel Context.
    pub fn blank() -> Self {
        Self {
            ksp: 0,
            ktp: 0,
            _sregs: [0; 12],
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
            sd      sp, 0*8(a0)
            sd      tp, 1*8(a0)
            sd      s0, 2*8(a0)
            sd      s1, 3*8(a0)
            sd      s2, 4*8(a0)
            sd      s3, 5*8(a0)
            sd      s4, 6*8(a0)
            sd      s5, 7*8(a0)
            sd      s6, 8*8(a0)
            sd      s7, 9*8(a0)
            sd      s8, 10*8(a0)
            sd      s9, 11*8(a0)
            sd      s10, 12*8(a0)
            sd      s11, 13*8(a0)
            sd      ra, 14*8(a0)
        ",
        // Restore Kernel Context.
        "
            ld      sp, 0*8(a1)
            ld      tp, 1*8(a1)
            ld      s0, 2*8(a1)
            ld      s1, 3*8(a1)
            ld      s2, 4*8(a1)
            ld      s3, 5*8(a1)
            ld      s4, 6*8(a1)
            ld      s5, 7*8(a1)
            ld      s6, 8*8(a1)
            ld      s7, 9*8(a1)
            ld      s8, 10*8(a1)
            ld      s9, 11*8(a1)
            ld      s10, 12*8(a1)
            ld      s11, 13*8(a1)
            ld      ra, 14*8(a1)
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
            sd      sp, 0*8(a0)
            sd      tp, 1*8(a0)
            sd      s0, 2*8(a0)
            sd      s1, 3*8(a0)
            sd      s2, 4*8(a0)
            sd      s3, 5*8(a0)
            sd      s4, 6*8(a0)
            sd      s5, 7*8(a0)
            sd      s6, 8*8(a0)
            sd      s7, 9*8(a0)
            sd      s8, 10*8(a0)
            sd      s9, 11*8(a0)
            sd      s10, 12*8(a0)
            sd      s11, 13*8(a0)
            sd      ra, 14*8(a0)
        ",
        // Switch to new page table.
        "
            srli    a2,   a2, 12
            li      a3,   8 << 60
            or      a2,   a2, a3
            csrw    satp, a2
            sfence.vma
        ",
        // Restore Kernel Context.
        "
            ld      sp, 0*8(a1)
            ld      tp, 1*8(a1)
            ld      s0, 2*8(a1)
            ld      s1, 3*8(a1)
            ld      s2, 4*8(a1)
            ld      s3, 5*8(a1)
            ld      s4, 6*8(a1)
            ld      s5, 7*8(a1)
            ld      s6, 8*8(a1)
            ld      s7, 9*8(a1)
            ld      s8, 10*8(a1)
            ld      s9, 11*8(a1)
            ld      s10, 12*8(a1)
            ld      s11, 13*8(a1)
            ld      ra, 14*8(a1)
            ret
        ",
        options(noreturn)
    )
}

#[naked]
pub extern "C" fn read_current_tp() -> usize {
    unsafe {
        asm!(
            "
                mv      a0, tp
                ret
            ",
            options(noreturn)
        )
    }
}
