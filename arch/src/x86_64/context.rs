use core::{fmt::Debug, ops::{Index, IndexMut}};

use x86_64::registers::rflags::RFlags;

use crate::ContextArgs;

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
pub struct Context {
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

impl Context {
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

impl Context {
    #[inline]
    pub fn args(&self) -> [usize; 6] {
        [self.rdi, self.rsi, self.rdx, self.r10, self.r8, self.r9]
    }

    #[inline]
    pub fn syscall_ok(&mut self) {
        // self.sepc += 4;
    }
}

// impl ContextOps for Context {
//     #[inline]
//     fn set_sp(&mut self, sp: usize) {
//         self.rsp = sp;
//     }

//     #[inline]
//     fn sp(&self) -> usize {
//         self.rsp
//     }
//     #[inline]
//     fn set_ra(&mut self, ra: usize) {
//         warn!("set_ra in x86_64 is push return address to rsp, shoule be execute at end");
//         self.rsp -= 8;
//         unsafe {
//             *(self.rsp as *mut usize) = ra;
//         }
//         // unimplemented!("set ra in x86_64 is not implemented")
//     }

//     #[inline]
//     fn ra(&self) -> usize {
//         unimplemented!("get ra in x86_64 is not implemented")
//     }

//     #[inline]
//     fn set_sepc(&mut self, sepc: usize) {
//         self.rip = sepc;
//     }

//     #[inline]
//     fn sepc(&self) -> usize {
//         self.rip
//     }

//     #[inline]
//     fn syscall_number(&self) -> usize {
//         self.rax
//     }

//     #[inline]
//     fn args(&self) -> [usize; 6] {
//         [self.rdi, self.rsi, self.rdx, self.r10, self.r8, self.r9]
//     }

//     #[inline]
//     fn syscall_ok(&mut self) {
//         // self.sepc += 4;
//     }

//     fn set_ret(&mut self, ret: usize) {
//         self.rax = ret;
//     }

//     fn set_arg0(&mut self, ret: usize) {
//         self.rdi = ret;
//     }

//     fn set_arg1(&mut self, ret: usize) {
//         self.rsi = ret;
//     }

//     fn set_arg2(&mut self, ret: usize) {
//         self.rdx = ret;
//     }

//     #[inline]
//     fn set_tls(&mut self, tls: usize) {
//         self.fs_base = tls;
//     }
// }

impl Context {
    #[inline]
    pub fn is_user(&self) -> bool {
        self.cs == GdtStruct::UCODE64_SELECTOR.0 as _
    }
}

impl Index<ContextArgs> for Context {
    type Output = usize;

    fn index(&self, index: ContextArgs) -> &Self::Output {
        match index {
            ContextArgs::SEPC => &self.rip,
            ContextArgs::RA => unimplemented!("Can't get return address in x86_64"),
            ContextArgs::ARG0 => &self.rdi,
            ContextArgs::ARG1 => &self.rsi,
            ContextArgs::ARG2 => &self.rdx,
            ContextArgs::TLS  => &self.fs_base,
            ContextArgs::SP => &self.rsp,
            ContextArgs::RET => &self.rax,
            ContextArgs::SYSCALL => &self.rax,
        }
    }
}

impl IndexMut<ContextArgs> for Context {
    fn index_mut(&mut self, index: ContextArgs) -> &mut Self::Output {
        match index {
            ContextArgs::SEPC => &mut self.rip,
            ContextArgs::RA => {
                // set return address, at x86_64 is push return address to rsp, shoule be execute at end.
                warn!("set_ra in x86_64 is push return address to rsp, shoule be execute at end");
                self.rsp -= 8;
                unsafe {
                    (self.rsp as *mut usize).as_mut().unwrap()
                }
            },
            ContextArgs::ARG0 => &mut self.rdi,
            ContextArgs::ARG1 => &mut self.rsi,
            ContextArgs::ARG2 => &mut self.rdx,
            ContextArgs::TLS  => &mut self.fs_base,
            ContextArgs::SP => &mut self.rsp,
            ContextArgs::RET => &mut self.rax,
            ContextArgs::SYSCALL => unreachable!("can't set syscall number")
        }
    }
}
