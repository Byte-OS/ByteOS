#[repr(C)]
#[derive(Debug, Clone)]
// 上下文
pub struct Context {
    pub x: [usize; 32], // 32 个通用寄存器
    pub sstatus: usize,
    pub sepc: usize,
}

impl Context {
    // 创建上下文信息
    #[inline]
    pub const fn new() -> Self {
        Context {
            x: [0usize; 32],
            sstatus: 0,
            sepc: 0,
        }
    }
    // 从另一个上下文复制
    #[inline]
    pub fn clone_from(&mut self, target: &Self) {
        for i in 0..32 {
            self.x[i] = target.x[i];
        }

        self.sstatus = target.sstatus;
        self.sepc = target.sepc;
    }
}


