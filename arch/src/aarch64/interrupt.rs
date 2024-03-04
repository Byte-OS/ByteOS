use crate::Context;
use crate::{add_irq, shutdown, TrapType, VIRT_ADDR_START};
use core::arch::{asm, global_asm};

// 设置中断
pub fn init_interrupt() {
    unsafe {
        asm!("brk #0");
    }
    enable_irq();
}

// 内核中断回调
#[no_mangle]
fn kernel_callback(context: &mut Context) -> usize {
    todo!("kernel_callback")
}

pub fn trap_pre_handle(context: &mut Context) -> TrapType {
    todo!("trap_pre_handle")
}

#[naked]
pub unsafe extern "C" fn kernelvec() {
    // asm!(
    //     // 宏定义
    //     r"
    //         .align 4
    //         .altmacro
    //         .set    REG_SIZE, 8
    //         .set    CONTEXT_SIZE, 34
    //     ",
    //     // 保存寄存器信息
    //     "   addi    sp, sp, CONTEXT_SIZE*-8
    //         sd      x1, 1*8(sp)
    //         addi    x1, sp, 34*8
    //         sd      x1, 2*8(sp)
    //         .set    n, 3
    //         .rept   29
    //             SAVE_N  %n
    //         .set    n, n + 1
    //         .endr
    //     ",
    //     r"  csrr    t0, sstatus
    //         csrr    t1, sepc
    //         sd      t0, 32*8(sp)
    //         sd      t1, 33*8(sp)
    //         add     a0, x0, sp",
    //     // 调用内核处理函数
    //     "   call kernel_callback ",
    //     // 恢复上下文信息
    //     "   ld      s1, 32*8(sp)
    //         ld      s2, 33*8(sp)
    //         csrw    sstatus, s1
    //         csrw    sepc, s2
    //         ld      x1, 1*8(sp)
    //         .set    n, 3
    //         .rept   29
    //             LOAD_N  %n
    //             .set    n, n + 1
    //         .endr
    //         ld      x2, 2*8(sp)
    //         sret
    //     ",
    //     options(noreturn)
    // )
    asm!("", options(noreturn))
}

#[naked]
#[no_mangle]
pub extern "C" fn user_restore(context: *mut Context) {
    // unsafe {
    //     asm!(r"
    //     .align 4
    //     .altmacro
    //     .set    REG_SIZE, 8
    //     .set    CONTEXT_SIZE, 34
    // ",
    //     // 在内核态栈中开一个空间来存储内核态信息
    //     // 下次发生中断必然会进入中断入口然后恢复这个上下文.
    //     // 仅保存 Callee-saved regs、gp、tp、ra.
    // "   addi    sp, sp, -18*8

    //     sd      sp, 8*1(sp)
    //     sd      gp, 8*2(sp)
    //     sd      tp, 8*3(sp)
    //     sd      s0, 8*4(sp)
    //     sd      s1, 8*5(sp)
    //     sd      s2, 8*6(sp)
    //     sd      s3, 8*7(sp)
    //     sd      s4, 8*8(sp)
    //     sd      s5, 8*9(sp)
    //     sd      s6, 8*10(sp)
    //     sd      s7, 8*11(sp)
    //     sd      s8, 8*12(sp)
    //     sd      s9, 8*13(sp)
    //     sd      s10, 8*14(sp)
    //     sd      s11, 8*15(sp)
    //     sd      a0,  8*16(sp)
    //     sd      ra,  8*17(sp)

    //     la a1, {uservec}
    //     csrw stvec, a1
    // ",
    //     // 将栈信息保存到用户栈.
    //     // a0 是传入的Context, 然后下面会再次恢复 sp 地址.
    // "   csrw    sscratch, sp
    //     mv      sp, a0

    //     LOAD    t0, 32
    //     LOAD    t1, 33
    //     .short 0x2452
    //     .short 0x24f2
    // ",
    //     // fld  fs0, 272(sp)
    //     // fld  fs1, 280(sp)
    // "
    //     csrw    sstatus, t0
    //     csrw    sepc, t1
    // ",
    //     // 恢复用户态通用寄存器x1, x3 - x31
    // r"  LOAD    x1, 1
    //     .set    n, 3
    //     .rept   29
    //         LOAD_N  %n
    //         .set    n, n + 1
    //     .endr",
    //     // 恢复 sp（又名 x2）这里最后恢复是为了上面可以正常使用 LOAD 宏
    // r"  LOAD    x2, 2
    //     sret
    // ",
    // uservec = sym uservec,
    // options(noreturn))
    // }
    unsafe { asm!("", options(noreturn)) }
}

#[naked]
#[no_mangle]
pub unsafe extern "C" fn uservec() {
    // asm!(r"
    //     .altmacro
    //     .align 2
    //     .set    REG_SIZE, 8
    //     .set    CONTEXT_SIZE, 34
    // ",
    //     // a0(x10) 是在 user_restore 时传入的 context 地址.
    //     // 因此将中断时用户的 寄存器存在这个地方
    // "   csrrw sp, sscratch, sp
    //     sd tp, 0(sp)
    //     ld tp, 16*8(sp) # 加载从x10保存的 context地址
    // ",
    //     // 保存 general registers, 除了 sp, tp
    // "   SAVE_TP_N 1
    //     SAVE_TP_N 3
    //     .set n, 5
    //     .rept 27
    //         SAVE_TP_N %n
    //         .set n, n+1
    //     .endr
    // ",
    //     // 保存特殊寄存器信息，sscratch 是用户 sp 地址.
    //     // 保存 sp 寄存器
    // "   csrr t0, sstatus
    //     csrr t1, sepc
    //     csrr t2, sscratch
    //     sd t0, 32*8(tp)
    //     sd t1, 33*8(tp)
    //     sd t2, 2*8(tp)
    //     .word 0x10823827
    //     .word 0x10923c27
    // ",
    //     // fsd fs0, 272(tp)
    //     // fsd fs1, 280(tp)
    //     // 保存 tp 寄存器，到此处所有的用户态寄存器已经保存
    // "   ld a0, 0(sp)
    //     sd a0, 4*8(tp)
    // ",
    //     // 恢复内核上下文信息, 仅恢复 callee-saved 寄存器和 ra、gp、tp
    // "
    //     ld      gp, 8*2(sp)
    //     ld      tp, 8*3(sp)
    //     ld      s0, 8*4(sp)
    //     ld      s1, 8*5(sp)
    //     ld      s2, 8*6(sp)
    //     ld      s3, 8*7(sp)
    //     ld      s4, 8*8(sp)
    //     ld      s5, 8*9(sp)
    //     ld      s6, 8*10(sp)
    //     ld      s7, 8*11(sp)
    //     ld      s8, 8*12(sp)
    //     ld      s9, 8*13(sp)
    //     ld      s10, 8*14(sp)
    //     ld      s11, 8*15(sp)
    //     ld      ra,  8*17(sp)

    //     ld      sp, 8(sp)

    //     la a0, {kernelvec}
    //     csrw stvec, a0
    // ",
    //     // 回收栈
    // "   addi sp, sp, 18*8
    //     ret
    // ",
    // kernelvec = sym kernelvec,
    // options(noreturn));
    asm!("", options(noreturn))
}

#[allow(dead_code)]
#[inline(always)]
pub fn enable_irq() {
    unsafe { asm!("msr daifclr, #2") };
}

#[inline(always)]
pub fn enable_external_irq() {
    // unsafe {
    //     sie::set_sext();
    // }
}
