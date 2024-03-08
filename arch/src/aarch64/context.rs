use crate::ContextOps;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct GeneralRegs {
    x0: usize,
    x1: usize,
    x2: usize,
    x3: usize,
    x4: usize,
    x5: usize,
    x6: usize,
    x7: usize,
    x8: usize,
    x9: usize,
    x10: usize,
    x11: usize,
    x12: usize,
    x13: usize,
    x14: usize,
    x15: usize,
    x16: usize,
    x17: usize,
    x18: usize,
    x19: usize,
    x20: usize,
    x21: usize,
    x22: usize,
    x23: usize,
    x24: usize,
    x25: usize,
    x26: usize,
    x27: usize,
    x28: usize,
    x29: usize,
    x30: usize,
}

/// Saved registers when a trap (interrupt or exception) occurs.
#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct Context {
    pub regs: GeneralRegs,
    pub sp: usize,
    pub elr: usize,
    pub spsr: usize,
    pub tpidr: usize,
}

impl Context {
    // 创建上下文信息
    #[inline]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl ContextOps for Context {
    #[inline]
    fn set_sp(&mut self, sp: usize) {
        self.sp = sp
    }

    #[inline]
    fn sp(&self) -> usize {
        self.sp
    }
    #[inline]
    fn set_ra(&mut self, _ra: usize) {
        unimplemented!("set ra in x86_64 is not implemented")
    }

    #[inline]
    fn ra(&self) -> usize {
        unimplemented!("get ra in x86_64 is not implemented")
    }

    #[inline]
    fn set_sepc(&mut self, sepc: usize) {
        self.elr = sepc;
    }

    #[inline]
    fn sepc(&self) -> usize {
        self.elr
    }

    #[inline]
    fn syscall_number(&self) -> usize {
        self.regs.x8
    }

    #[inline]
    fn args(&self) -> [usize; 6] {
        [
            self.regs.x0,
            self.regs.x1,
            self.regs.x2,
            self.regs.x3,
            self.regs.x4,
            self.regs.x5,
        ]
    }

    #[inline]
    fn syscall_ok(&mut self) {
        // self.sepc += 4;
    }

    fn set_ret(&mut self, ret: usize) {
        self.regs.x0 = ret;
    }

    fn set_arg0(&mut self, ret: usize) {
        self.regs.x0 = ret;
    }

    fn set_arg1(&mut self, ret: usize) {
        self.regs.x1 = ret;
    }

    fn set_arg2(&mut self, ret: usize) {
        self.regs.x2 = ret;
    }

    #[inline]
    fn set_tls(&mut self, tls: usize) {
        self.tpidr = tls
    }
}
