use core::arch::asm;

use riscv::register::scause::{Exception, Scause, Trap};

use crate::{riscv64::context::Context, shutdown};

// 设置中断
pub fn init_interrupt() {
    // 输出内核信息
    unsafe {
        asm!("csrw stvec, a0", in("a0") kernelvec as usize);

        // 测试
        info!("测试 ebreak exception");
        asm!("ebreak");
    }

    // // 初始化定时器
    // timer::init();
}

// 内核中断回调
#[no_mangle]
fn kernel_callback(context: &mut Context, scause: Scause, stval: usize) -> usize {
    warn!(
        "内核态中断发生: {:#x} {:?}  stval {:#x}  sepc: {:#x}",
        scause.bits(),
        scause.cause(),
        stval,
        context.sepc
    );
    match scause.cause() {
        // 中断异常
        Trap::Exception(Exception::Breakpoint) => context.sepc += 2,
        Trap::Exception(Exception::LoadFault) => {
            shutdown();
        }
        // // 时钟中断 eg: 不再内核处理时间中断 just in user
        // Trap::Interrupt(Interrupt::SupervisorTimer) => {},
        // // 缺页异常
        // Trap::Exception(Exception::StorePageFault) => handle_page_fault(context, stval),
        // // 加载页面错误
        // Trap::Exception(Exception::LoadPageFault) => {
        //     panic!("加载权限异常 地址:{:#x}", stval)
        // },
        // Trap::Exception(Exception::InstructionPageFault) => handle_page_fault(context, stval),
        // // 页面未对齐异常
        // Trap::Exception(Exception::StoreMisaligned) => {
        //     info!("页面未对齐");
        // }
        // 其他情况，终止当前线程
        _ => {
            warn!("未知中断");
        }
    }
    context as *const Context as usize
}

#[naked]
pub unsafe extern "C" fn kernelvec() {
    asm!(
        // 宏定义
        r"
        .align 4
        .altmacro
        .set    REG_SIZE, 8
        .set    CONTEXT_SIZE, 34
        
        .macro SAVE_N n
            sd  x\n, \n*8(sp)
        .endm
        
        .macro LOAD_N n
            ld  x\n, \n*8(sp)
        .endm ",
        // 保存寄存器信息
        "   addi    sp, sp, CONTEXT_SIZE*-8
        sd      x1, 1*8(sp)
        addi    x1, sp, 34*8
        sd      x1, 2*8(sp)
        .set    n, 3
        .rept   29 
            SAVE_N  %n
        .set    n, n + 1
        .endr

        csrr    t0, sstatus
        csrr    t1, sepc
        sd      t0, 32*8(sp)
        sd      t1, 33*8(sp)
        add a0, x0, sp
        csrr a1, scause
        csrr a2, stval ",
        // 调用内核处理函数
        "   call kernel_callback ",
        // 恢复上下文信息
        "   ld      s1, 32*8(sp)
        ld      s2, 33*8(sp)
        csrw    sstatus, s1
        csrw    sepc, s2
        ld      x1, 1*8(sp)
        .set    n, 3
        .rept   29
            LOAD_N  %n
            .set    n, n + 1
        .endr
        ld      x2, 2*8(sp)
        sret
    ",
        options(noreturn)
    )
}
