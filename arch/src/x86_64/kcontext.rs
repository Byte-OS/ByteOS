use core::ops::{Index, IndexMut};

use x86_64::registers::model_specific::FsBase;

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
    // Callee saved register
    rbx: usize,
    // Callee saved register
    rbp: usize,
    // Callee saved register
    r12: usize,
    // Callee saved register
    r13: usize,
    // Callee saved register
    r14: usize,
    // Callee saved register
    r15: usize,
    /// Kernel Program Counter, Will return to this address.
    kpc: usize,
}

impl KContext {
    /// Create a new blank Kernel Context.
    pub fn blank() -> Self {
        Self {
            ksp: 0,
            ktp: 0,
            rbx: 0,
            rbp: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
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
            
        ",
        // Restore Kernel Context.
        "
            
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
        // consume the return address(rip) in the stack
        // for consistency with context_switch.
        // and save page table to r9
        "
            pop     r8
            mov     r9, rdx
        ",
        // Save Kernel Context.
        "
            mov     [rdi + 0 * 8], rsp
            mov     [rdi + 2 * 8], rbx
            mov     [rdi + 3 * 8], rbp
            mov     [rdi + 4 * 8], r12
            mov     [rdi + 5 * 8], r13
            mov     [rdi + 6 * 8], r14
            mov     [rdi + 7 * 8], r15
            mov     [rdi + 8 * 8], r8     # save old rip to stack

            mov     ecx, 0xC0000100
            rdmsr
            mov     [rdi + 1*8],    eax   # push fabase
            mov     [rdi + 1*8+4],  edx
        ",
        // Switch to new page table.
        "
            mov     cr3,   r9
        ",
        // Restore Kernel Context.
        "
            mov     ecx, 0xC0000100
            mov     eax, [rsi + 1*8]
            mov     edx, [rsi + 1*8+4]
            wrmsr                         # pop fsbase
            mov     rsp, [rsi + 0 * 8]
            mov     rbx, [rsi + 2 * 8]
            mov     rbp, [rsi + 3 * 8]
            mov     r12, [rsi + 4 * 8]
            mov     r13, [rsi + 5 * 8]
            mov     r14, [rsi + 6 * 8]
            mov     r15, [rsi + 7 * 8]
            mov     r8,  [rsi + 8 * 8]
            
            push    r8
            ret
        ",
        options(noreturn)
    )
}

/// Read thread pointer currently.
#[inline]
pub fn read_current_tp() -> usize {
    FsBase::read().as_u64() as _
}
