use core::{
    fmt::Debug,
    ops::{Index, IndexMut},
};

use x86_64::registers::rflags::RFlags;

use crate::TrapFrameArgs;

use super::gdt::GdtStruct;

#[repr(C, align(16))]
#[derive(Clone)]
pub struct FxsaveArea {
    pub fcw: u16,
    pub fsw: u16,
    pub ftw: u16,
    pub fop: u16,
    pub fip: u64,
    pub fdp: u64,
    pub mxcsr: u32,
    pub mxcsr_mask: u32,
    pub st: [u64; 16],
    pub xmm: [u64; 32],
    _padding: [u64; 12],
}

impl FxsaveArea {
    #[inline]
    pub(crate) fn save(&mut self) {
        unsafe { core::arch::x86_64::_fxsave64(self as *mut _ as *mut u8) }
    }

    #[inline]
    pub(crate) fn restore(&self) {
        unsafe { core::arch::x86_64::_fxrstor64(self as *const _ as *const u8) }
    }
}

impl Default for FxsaveArea {
    fn default() -> Self {
        let mut area: FxsaveArea = unsafe { core::mem::MaybeUninit::zeroed().assume_init() };
        area.fcw = 0x37f;
        area.ftw = 0xffff;
        area.mxcsr = 0x1f80;
        area
    }
}

impl Debug for FxsaveArea {
    fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}

/// Saved registers when a trap (interrupt or exception) occurs.
/// This is need be align 16, because tss trap ptr should be align 16? I think it is.
#[allow(missing_docs)]
#[repr(C, align(16))]
#[derive(Debug, Default, Clone)]
pub struct TrapFrame {
    pub rax: usize,
    pub rcx: usize,
    pub rdx: usize,
    pub rbx: usize,
    pub rbp: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub r8: usize,
    pub r9: usize,
    pub r10: usize,
    pub r11: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,

    pub fs_base: usize,
    pub gs_base: usize,

    // Pushed by `trap.S`
    pub vector: usize,
    pub error_code: usize,

    // Pushed by CPU
    pub rip: usize,
    pub cs: usize,
    pub rflags: usize,
    pub rsp: usize,
    pub ss: usize,

    // save fx area
    pub fx_area: FxsaveArea,
}

impl TrapFrame {
    // 创建上下文信息
    #[inline]
    pub fn new() -> Self {
        debug!(
            "new_user cs: {:#x}, ss: {:#x}",
            GdtStruct::UCODE64_SELECTOR.0,
            GdtStruct::UDATA_SELECTOR.0
        );
        Self {
            cs: GdtStruct::UCODE64_SELECTOR.0 as _,
            ss: GdtStruct::UDATA_SELECTOR.0 as _,
            rflags: RFlags::INTERRUPT_FLAG.bits() as _,
            ..Default::default()
        }
    }
}

impl TrapFrame {
    #[inline]
    pub fn args(&self) -> [usize; 6] {
        [self.rdi, self.rsi, self.rdx, self.r10, self.r8, self.r9]
    }

    #[inline]
    pub fn syscall_ok(&mut self) {
        // self.sepc += 4;
    }

    #[inline]
    pub fn is_user(&self) -> bool {
        self.cs == GdtStruct::UCODE64_SELECTOR.0 as _
    }
}

impl Index<TrapFrameArgs> for TrapFrame {
    type Output = usize;

    fn index(&self, index: TrapFrameArgs) -> &Self::Output {
        match index {
            TrapFrameArgs::SEPC => &self.rip,
            TrapFrameArgs::RA => unimplemented!("Can't get return address in x86_64"),
            TrapFrameArgs::ARG0 => &self.rdi,
            TrapFrameArgs::ARG1 => &self.rsi,
            TrapFrameArgs::ARG2 => &self.rdx,
            TrapFrameArgs::TLS => &self.fs_base,
            TrapFrameArgs::SP => &self.rsp,
            TrapFrameArgs::RET => &self.rax,
            TrapFrameArgs::SYSCALL => &self.rax,
        }
    }
}

impl IndexMut<TrapFrameArgs> for TrapFrame {
    fn index_mut(&mut self, index: TrapFrameArgs) -> &mut Self::Output {
        match index {
            TrapFrameArgs::SEPC => &mut self.rip,
            TrapFrameArgs::RA => {
                // set return address, at x86_64 is push return address to rsp, shoule be execute at end.
                warn!("set_ra in x86_64 is push return address to rsp, shoule be execute at end");
                self.rsp -= 8;
                unsafe { (self.rsp as *mut usize).as_mut().unwrap() }
            }
            TrapFrameArgs::ARG0 => &mut self.rdi,
            TrapFrameArgs::ARG1 => &mut self.rsi,
            TrapFrameArgs::ARG2 => &mut self.rdx,
            TrapFrameArgs::TLS => &mut self.fs_base,
            TrapFrameArgs::SP => &mut self.rsp,
            TrapFrameArgs::RET => &mut self.rax,
            TrapFrameArgs::SYSCALL => unreachable!("can't set syscall number"),
        }
    }
}
