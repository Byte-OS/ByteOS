use core::arch::{asm, global_asm};

use crate::{Context, TrapType};

global_asm!(
    r"
    .altmacro
    .macro LOAD reg, offset
        ld  \reg, \offset*8(sp)
    .endm

    .macro SAVE reg, offset
        sd  \reg, \offset*8(sp)
    .endm

    .macro LOAD_N n
        ld  x\n, \n*8(sp)
    .endm

    .macro SAVE_N n
        sd  x\n, \n*8(sp)
    .endm

    .macro SAVE_TP_N n
        sd  x\n, \n*8(tp)
    .endm
"
);

// 内核中断回调
#[no_mangle]
fn kernel_callback(context: &mut Context) -> usize {
    // let scause = scause::read();
    // let stval = stval::read();
    // let int_table = unsafe { interrupt_table() };
    // debug!(
    //     "内核态中断发生: {:#x} {:?}  stval {:#x}  sepc: {:#x}",
    //     scause.bits(),
    //     scause.cause(),
    //     stval,
    //     context.sepc
    // );
    // let trap_type = match scause.cause() {
    //     // 中断异常
    //     Trap::Exception(Exception::Breakpoint) => {
    //         context.sepc += 2;
    //         TrapType::Breakpoint
    //     }
    //     Trap::Exception(Exception::LoadFault) => {
    //         if stval > VIRT_ADDR_START {
    //             panic!("kernel error: {:#x}", stval);
    //         }
    //         TrapType::Unknown
    //     }
    //     // 时钟中断
    //     Trap::Interrupt(Interrupt::SupervisorTimer) => {
    //         timer::set_next_timeout();
    //         add_irq(5);
    //         TrapType::Time
    //     }
    //     Trap::Exception(Exception::UserEnvCall) => {
    //         info!("info syscall: {}", context.x[17]);
    //         context.sepc += 4;
    //         TrapType::UserEnvCall
    //     }
    //     Trap::Interrupt(Interrupt::SupervisorExternal) => TrapType::SupervisorExternal,
    //     // // 缺页异常
    //     // Trap::Exception(Exception::StorePageFault) => handle_page_fault(context, stval),
    //     // // 加载页面错误
    //     // Trap::Exception(Exception::LoadPageFault) => {
    //     //     panic!("加载权限异常 地址:{:#x}", stval)
    //     // },
    //     // Trap::Exception(Exception::InstructionPageFault) => handle_page_fault(context, stval),
    //     // // 页面未对齐异常
    //     // Trap::Exception(Exception::StoreMisaligned) => {
    //     //     info!("页面未对齐");
    //     // }
    //     // 其他情况，终止当前线程
    //     Trap::Exception(Exception::StorePageFault) => TrapType::StorePageFault(stval),
    //     Trap::Exception(Exception::InstructionPageFault) => TrapType::InstructionPageFault(stval),
    //     Trap::Exception(Exception::IllegalInstruction) => TrapType::IllegalInstruction(stval),
    //     _ => {
    //         // warn!("未知中断");
    //         error!(
    //             "内核态中断发生: {:#x} {:?}  stval {:#x}  sepc: {:#x}",
    //             scause.bits(),
    //             scause.cause(),
    //             stval,
    //             context.sepc
    //         );
    //         panic!("未知中断")
    //     }
    // };
    // if let Some(func) = int_table {
    //     func(context, trap_type);
    // }
    context as *const Context as usize
}

pub fn trap_pre_handle(context: &mut Context) -> TrapType {
    // let scause = scause::read();
    // let stval = stval::read();
    // debug!(
    //     "用户态中断发生: {:#x} {:?}  stval {:#x}  sepc: {:#x}",
    //     scause.bits(),
    //     scause.cause(),
    //     stval,
    //     context.sepc
    // );
    // match scause.cause() {
    //     // 中断异常
    //     Trap::Exception(Exception::Breakpoint) => {
    //         context.sepc += 2;
    //         TrapType::Breakpoint
    //     }
    //     Trap::Exception(Exception::LoadFault) => {
    //         shutdown();
    //     }
    //     // 时钟中断
    //     Trap::Interrupt(Interrupt::SupervisorTimer) => {
    //         timer::set_next_timeout();
    //         add_irq(5);
    //         TrapType::Time
    //     }
    //     Trap::Exception(Exception::StorePageFault) => TrapType::StorePageFault(stval),
    //     Trap::Exception(Exception::InstructionPageFault) => TrapType::InstructionPageFault(stval),
    //     Trap::Exception(Exception::IllegalInstruction) => {
    //         TrapType::IllegalInstruction(context.sepc)
    //     }
    //     Trap::Exception(Exception::UserEnvCall) => TrapType::UserEnvCall,
    //     Trap::Exception(Exception::LoadPageFault) => TrapType::LoadPageFault(stval),
    //     Trap::Interrupt(Interrupt::SupervisorExternal) => TrapType::SupervisorExternal,
    //     _ => {
    //         error!(
    //             "用户态中断发生: {:#x} {:?}  stval {:#x}  sepc: {:#x}",
    //             scause.bits(),
    //             scause.cause(),
    //             stval,
    //             context.sepc
    //         );
    //         TrapType::Unknown
    //     }
    // }
    todo!("todo trap_pre_handle")
}

#[naked]
pub unsafe extern "C" fn kernelvec() {
    asm!(
        // 宏定义
        // r"
        //     .align 4
        //     .altmacro
        //     .set    REG_SIZE, 8
        //     .set    CONTEXT_SIZE, 34
        // ",
        // // 保存寄存器信息
        // "   addi    sp, sp, CONTEXT_SIZE*-8
        //     sd      x1, 1*8(sp)
        //     addi    x1, sp, 34*8
        //     sd      x1, 2*8(sp)
        //     .set    n, 3
        //     .rept   29
        //         SAVE_N  %n
        //     .set    n, n + 1
        //     .endr
        // ",
        // r"  csrr    t0, sstatus
        //     csrr    t1, sepc
        //     sd      t0, 32*8(sp)
        //     sd      t1, 33*8(sp)
        //     add     a0, x0, sp",
        // // 调用内核处理函数
        // "   call kernel_callback ",
        // // 恢复上下文信息
        // "   ld      s1, 32*8(sp)
        //     ld      s2, 33*8(sp)
        //     csrw    sstatus, s1
        //     csrw    sepc, s2
        //     ld      x1, 1*8(sp)
        //     .set    n, 3
        //     .rept   29
        //         LOAD_N  %n
        //         .set    n, n + 1
        //     .endr
        //     ld      x2, 2*8(sp)
        //     sret
        // ",
        "",
        options(noreturn)
    )
}

#[naked]
#[no_mangle]
pub extern "C" fn user_restore(context: *mut Context) {
    unsafe {
        asm!(
            // Save callee saved registers and cs and others.
            r"
                push rbp
                push rbx
                push r12
                push r13
                push r14
                push r15
                mov rax, cs
                push rax
                mov rcx, ss
                push rcx
            ",
            // push fs_base
            "
                mov ecx, 0xC0000100
                rdmsr
                mov [rsp + 18*8+4], edx
                mov [rsp + 18*8], eax   # push fabase
            ",
            options(noreturn)
        )
    }
}

#[naked]
#[no_mangle]
pub unsafe extern "C" fn uservec() {
    asm!(
        // r"
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
        "",
        options(noreturn)
    );
}

#[allow(dead_code)]
#[inline(always)]
pub fn enable_irq() {
    unsafe { asm!("sti") }
}

pub fn close_irq() {
    unsafe { asm!("cli") }
}

#[inline(always)]
pub fn enable_external_irq() {
    // unsafe {

    // }
}

pub fn init_interrupt() {
    enable_irq()
}

pub fn time_to_usec(tiscks: usize) -> usize {
    tiscks
}

pub fn get_time() -> usize {
    unsafe { core::arch::x86_64::_rdtsc() as _ }
}

pub fn get_time_ms() -> usize {
    unsafe { core::arch::x86_64::_rdtsc() as _ }
}
