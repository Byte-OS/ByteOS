use polyhal_trap::trapframe::TrapFrame;
use signal::SigProcMask;

bitflags! {
    #[derive(Debug, Clone)]
    pub struct SignalStackFlags : u32 {
        const ONSTACK = 1;
        const DISABLE = 2;
        const AUTODISARM = 0x80000000;
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct SignalStack {
    pub sp: usize,
    pub flags: SignalStackFlags,
    pub size: usize,
}

cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        #[repr(C)]
        #[derive(Debug, Clone)]
        pub struct SignalUserContext {
            pub flags: usize,          // 0
            pub link: usize,           // 1
            pub stack: SignalStack,    // 2
            pub gregs: [usize; 32],
            pub sig_mask: SigProcMask, // sigmask
            pub _pad: [u64; 16],       // sigmask extend
            pub __fpregs_mem: [u64; 64]
        }

        impl SignalUserContext {
            pub fn pc(&self) -> usize {
                self.gregs[16]
            }

            pub fn set_pc(&mut self, v: usize) {
                self.gregs[16] = v;
            }

            pub fn store_ctx(&mut self, ctx: &TrapFrame) {
                self.gregs[0] = ctx.r8;
                self.gregs[1] = ctx.r9;
                self.gregs[2] = ctx.r10;
                self.gregs[3] = ctx.r11;
                self.gregs[4] = ctx.r12;
                self.gregs[5] = ctx.r13;
                self.gregs[6] = ctx.r14;
                self.gregs[7] = ctx.r15;
                self.gregs[8] = ctx.rdi;
                self.gregs[9] = ctx.rsi;
                self.gregs[10] = ctx.rbp;
                self.gregs[11] = ctx.rbx;
                self.gregs[12] = ctx.rdx;
                self.gregs[13] = ctx.rax;
                self.gregs[14] = ctx.rcx;
                self.gregs[15] = ctx.rsp;
                self.gregs[16] = ctx.rip;
            }

            pub fn restore_ctx(&self, ctx: &mut TrapFrame) {
                ctx.r8  = self.gregs[0];
                ctx.r9  = self.gregs[1];
                ctx.r10 = self.gregs[2];
                ctx.r11 = self.gregs[3];
                ctx.r12 = self.gregs[4];
                ctx.r13 = self.gregs[5];
                ctx.r14 = self.gregs[6];
                ctx.r15 = self.gregs[7];
                ctx.rdi = self.gregs[8];
                ctx.rsi = self.gregs[9];
                ctx.rbp = self.gregs[10];
                ctx.rbx = self.gregs[11];
                ctx.rdx = self.gregs[12];
                ctx.rax = self.gregs[13];
                ctx.rcx = self.gregs[14];
                ctx.rsp = self.gregs[15];
                ctx.rip = self.gregs[16];
            }
        }
    } else if #[cfg(target_arch = "riscv64")] {
        #[repr(C)]
        #[derive(Debug, Clone)]
        pub struct SignalUserContext {
            pub flags: usize,          // 0
            pub link: usize,           // 1
            pub stack: SignalStack,    // 2
            pub sig_mask: SigProcMask, // 5
            pub _pad: [u64; 16],       // mask
            // pub context: Context,       // pc offset = 22 - 6=16
            pub gregs: [usize; 32],
            pub fpstate: [usize; 66],
        }

        impl SignalUserContext {
            pub fn pc(&self) -> usize {
                self.gregs[0]
            }

            pub fn set_pc(&mut self, v: usize) {
                self.gregs[0] = v;
            }

            pub fn store_ctx(&mut self, ctx: &TrapFrame) {
                self.gregs = ctx.x;
            }

            pub fn restore_ctx(&self, ctx: &mut TrapFrame) {
                ctx.x = self.gregs;
            }
        }
    } else if #[cfg(target_arch = "aarch64")] {
        #[repr(C)]
        #[derive(Debug, Clone)]
        pub struct SignalUserContext {
            pub flags: usize,          // 0
            pub link: usize,           // 1
            pub stack: SignalStack,    // 2
            pub sig_mask: SigProcMask, // 5
            pub _pad: [u64; 16],       // mask
            pub fault_address: usize,
            pub regs: [usize; 31],
            pub sp: usize,
            pub pc: usize,
            pub pstate: usize,
            pub __reserved: usize,
        }

        impl SignalUserContext {
            pub fn pc(&self) -> usize {
                self.pc
            }

            pub fn set_pc(&mut self, v: usize) {
                self.pc = v;
            }

            pub fn store_ctx(&mut self, ctx: &TrapFrame) {
                self.regs = ctx.regs;
            }

            pub fn restore_ctx(&self, ctx: &mut TrapFrame) {
                ctx.regs = self.regs;
            }
        }
    } else if #[cfg(target_arch = "loongarch64")] {
        #[repr(C)]
        #[derive(Debug, Clone)]
        pub struct SignalUserContext {
            pub flags: usize,          // 0
            pub link: usize,           // 1
            pub stack: SignalStack,    // 2
            pub sig_mask: SigProcMask, // 5
            pub _pad: [u64; 2],       // mask
            pub pc: usize,
            pub gregs: [usize; 32],
            pub gflags: u32,
            pub fcsr: u32,
            pub scr: [usize; 4],
            pub fregs: [usize; 32],        // _extcontext
            pub _reserved: [usize; 512],
        }

        impl SignalUserContext {
            pub fn pc(&self) -> usize {
                self.pc
            }

            pub fn set_pc(&mut self, v: usize) {
                self.pc = v;
            }

            pub fn store_ctx(&mut self, ctx: &TrapFrame) {
                self.gregs = ctx.regs;
            }

            pub fn restore_ctx(&self, ctx: &mut TrapFrame) {
                ctx.regs = self.gregs;
            }
        }
    }
}
