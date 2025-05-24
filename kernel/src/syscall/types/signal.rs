use libc_types::{signal::UContext, types::SigSet};
use polyhal_trap::trapframe::TrapFrame;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct SignalUserContext(UContext);

#[cfg(target_arch = "x86_64")]
impl SignalUserContext {
    pub const fn pc(&self) -> usize {
        self.0.gregs.rip
    }

    pub const fn set_pc(&mut self, v: usize) {
        self.0.gregs.rip = v;
    }

    pub const fn store_ctx(&mut self, ctx: &TrapFrame) {
        self.0.gregs.r8 = ctx.r8;
        self.0.gregs.r9 = ctx.r9;
        self.0.gregs.r10 = ctx.r10;
        self.0.gregs.r11 = ctx.r11;
        self.0.gregs.r12 = ctx.r12;
        self.0.gregs.r13 = ctx.r13;
        self.0.gregs.r14 = ctx.r14;
        self.0.gregs.r15 = ctx.r15;
        self.0.gregs.rdi = ctx.rdi;
        self.0.gregs.rsi = ctx.rsi;
        self.0.gregs.rbp = ctx.rbp;
        self.0.gregs.rbx = ctx.rbx;
        self.0.gregs.rdx = ctx.rdx;
        self.0.gregs.rax = ctx.rax;
        self.0.gregs.rcx = ctx.rcx;
        self.0.gregs.rsp = ctx.rsp;
        self.0.gregs.rip = ctx.rip;
    }

    pub const fn restore_ctx(&self, ctx: &mut TrapFrame) {
        ctx.r8 = self.0.gregs.r8;
        ctx.r9 = self.0.gregs.r9;
        ctx.r10 = self.0.gregs.r10;
        ctx.r11 = self.0.gregs.r11;
        ctx.r12 = self.0.gregs.r12;
        ctx.r13 = self.0.gregs.r13;
        ctx.r14 = self.0.gregs.r14;
        ctx.r15 = self.0.gregs.r15;
        ctx.rdi = self.0.gregs.rdi;
        ctx.rsi = self.0.gregs.rsi;
        ctx.rbp = self.0.gregs.rbp;
        ctx.rbx = self.0.gregs.rbx;
        ctx.rdx = self.0.gregs.rdx;
        ctx.rax = self.0.gregs.rax;
        ctx.rcx = self.0.gregs.rcx;
        ctx.rsp = self.0.gregs.rsp;
        ctx.rip = self.0.gregs.rip;
    }

    pub const fn set_sig_mask(&mut self, sigset: SigSet) {
        self.0.sig_mask.sigset = sigset;
    }
}

#[cfg(any(target_arch = "riscv64"))]
impl SignalUserContext {
    pub const fn pc(&self) -> usize {
        self.0.regs.gregs[0]
    }

    pub const fn set_pc(&mut self, v: usize) {
        self.0.regs.gregs[0] = v;
    }

    pub const fn store_ctx(&mut self, ctx: &TrapFrame) {
        let mut i = 1;
        while i < 32 {
            self.0.regs.gregs[i] = ctx.x[i];
            i += 1;
        }
    }

    pub const fn restore_ctx(&self, ctx: &mut TrapFrame) {
        let mut i = 1;
        while i < 32 {
            ctx.x[i] = self.0.regs.gregs[i];
            i += 1;
        }
    }

    pub const fn set_sig_mask(&mut self, sigset: SigSet) {
        self.0.sig_mask.sigset = sigset;
    }
}

#[cfg(target_arch = "aarch64")]
impl SignalUserContext {
    pub const fn pc(&self) -> usize {
        self.0.regs.pc
    }

    pub const fn set_pc(&mut self, v: usize) {
        self.0.regs.pc = v;
    }

    pub const fn store_ctx(&mut self, ctx: &TrapFrame) {
        let mut i = 0;
        while i < 31 {
            self.0.regs.gregs[i] = ctx.regs[i];
            i += 1;
        }
    }

    pub const fn restore_ctx(&self, ctx: &mut TrapFrame) {
        let mut i = 0;
        while i < 31 {
            ctx.regs[i] = self.0.regs.gregs[i];
            i += 1;
        }
    }

    pub const fn set_sig_mask(&mut self, sigset: SigSet) {
        self.0.sig_mask.sigset = sigset;
    }
}

#[cfg(target_arch = "loongarch64")]
impl SignalUserContext {
    pub const fn pc(&self) -> usize {
        self.0.regs.pc
    }

    pub const fn set_pc(&mut self, v: usize) {
        self.0.regs.pc = v;
    }

    pub const fn store_ctx(&mut self, ctx: &TrapFrame) {
        let mut i = 0;
        while i < 32 {
            self.0.regs.gregs[i] = ctx.regs[i];
            i += 1;
        }
    }

    pub const fn restore_ctx(&self, ctx: &mut TrapFrame) {
        let mut i = 0;
        while i < 32 {
            ctx.regs[i] = self.0.regs.gregs[i];
            i += 1;
        }
    }

    pub const fn set_sig_mask(&mut self, sigset: SigSet) {
        self.0.sig_mask.sigset = sigset;
    }
}
