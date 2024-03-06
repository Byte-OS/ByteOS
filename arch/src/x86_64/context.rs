use crate::ContextOps;

/// Saved registers when a trap (interrupt or exception) occurs.
#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
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

    // pub fs_base: usize,
    // pub gs_base: usize,

    // Pushed by `trap.S`
    pub vector: usize,
    pub error_code: usize,

    // Pushed by CPU
    pub rip: usize,
    pub cs: usize,
    pub rflags: usize,
    pub rsp: usize,
    pub ss: usize,
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
        self.rsp = sp;
    }

    #[inline]
    fn sp(&self) -> usize {
        self.rsp
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
        self.rip = sepc;
    }

    #[inline]
    fn sepc(&self) -> usize {
        self.rip
    }

    #[inline]
    fn syscall_number(&self) -> usize {
        self.rax
    }

    #[inline]
    fn args(&self) -> [usize; 6] {
        [self.rdi, self.rsi, self.rdx, self.r10, self.r8, self.r9]
    }

    #[inline]
    fn syscall_ok(&mut self) {
        // self.sepc += 4;
    }

    fn set_ret(&mut self, ret: usize) {
        self.rax = ret;
    }

    fn set_arg0(&mut self, ret: usize) {
        self.rdi = ret;
    }

    fn set_arg1(&mut self, ret: usize) {
        self.rsi = ret;
    }

    fn set_arg2(&mut self, ret: usize) {
        self.rdx = ret;
    }

    fn clear(&mut self) {
        *self = Default::default();
    }

    #[inline]
    fn set_tls(&mut self, _tls: usize) {
        unimplemented!("set tls is unimplemented!")
    }
}
