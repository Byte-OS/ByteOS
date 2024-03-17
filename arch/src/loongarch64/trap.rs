use core::arch::asm;

use loongarch64::register::{
    ecfg, eentry,
    estat::{self, Exception, Trap},
};

use super::Context;

pub fn init() {
    todo!("init interrupt")
}

// 设置中断
pub fn init_interrupt() {
    todo!("test brk");
    enable_irq();
}

#[naked]
#[no_mangle]
pub extern "C" fn user_restore(context: *mut Context) {
    unsafe {
        asm!(
            r"
            
        ",
            options(noreturn)
        )
    }
}

#[allow(dead_code)]
#[inline(always)]
pub fn enable_irq() {
    todo!("enable_irq")
}

#[inline(always)]
pub fn enable_external_irq() {
    // unsafe {
    //     sie::set_sext();
    // }
}

pub fn run_user_task(cx: &mut Context) -> Option<()> {
    todo!("run_user_task");
}

#[naked]
pub unsafe extern "C" fn trap_vector_base() {
    core::arch::asm!(
        "
            .balign 4096
            .equ KSAVE_KSP, 0x30
            .equ KSAVE_T0,  0x31
            .equ KSAVE_USP, 0x32
                // csrwr   $t0, KSAVE_T0
                // csrrd   $t0, 0x1
                // andi    $t0, $t0, 0x3
                // bnez    $t0, .Lfrom_userspace 
            
                move    $t0, $sp  
                addi.d  $sp, $sp, -{trapframe_size} // allocate space
                // save kernel sp
                st.d    $t0, $sp, 3*8
            
                // save the registers.
                st.d    $ra, $sp, 8
                csrrd   $t0, KSAVE_T0
                st.d    $t0, $sp, 12*8

                st.d    $a0, $sp, 4*8
                st.d    $a1, $sp, 5*8
                st.d    $a2, $sp, 6*8
                st.d    $a3, $sp, 7*8
                st.d    $a4, $sp, 8*8
                st.d    $a5, $sp, 9*8
                st.d    $a6, $sp, 10*8
                st.d    $a7, $sp, 11*8
                st.d    $t1, $sp, 13*8
                st.d    $t2, $sp, 14*8
                st.d    $t3, $sp, 15*8
                st.d    $t4, $sp, 16*8
                st.d    $t5, $sp, 17*8
                st.d    $t6, $sp, 18*8
                st.d    $t7, $sp, 19*8
                st.d    $t8, $sp, 20*8

                st.d    $fp, $sp, 22*8
                st.d    $s0, $sp, 23*8
                st.d    $s1, $sp, 24*8
                st.d    $s2, $sp, 25*8
                st.d    $s3, $sp, 26*8
                st.d    $s4, $sp, 27*8
                st.d    $s5, $sp, 28*8
                st.d    $s6, $sp, 29*8
                st.d    $s7, $sp, 30*8
                st.d    $s8, $sp, 31*8
            
                csrrd	$t2, 0x1
                st.d	$t2, $sp, 8*32  // prmd
                csrrd   $t1, 0x6        
                st.d    $t1, $sp, 8*33  // era
                csrrd   $t1, 0x7   
                st.d    $t1, $sp, 8*34  // badv  
                csrrd   $t1, 0x0   
                st.d    $t1, $sp, 8*35  // crmd    
            
                move    $a0, $sp
                csrrd   $t0, 0x1
                andi    $a1, $t0, 0x3   // if user or kernel
                bl      {trap_handler}
            
                // restore the registers.
                ld.d    $t1, $sp, 8*33  // era
                csrwr   $t1, 0x6
                ld.d    $t2, $sp, 8*32  // prmd
                csrwr   $t2, 0x1
            
                // Save kernel sp when exit kernel mode
                addi.d  $t1, $sp, {trapframe_size}
                csrwr   $t1, KSAVE_KSP 

                ld.d    $ra, $sp, 1*8
                ld.d    $a0, $sp, 4*8
                ld.d    $a1, $sp, 5*8
                ld.d    $a2, $sp, 6*8
                ld.d    $a3, $sp, 7*8
                ld.d    $a4, $sp, 8*8
                ld.d    $a5, $sp, 9*8
                ld.d    $a6, $sp, 10*8
                ld.d    $a7, $sp, 11*8
                ld.d    $t0, $sp, 12*8
                ld.d    $t1, $sp, 13*8
                ld.d    $t2, $sp, 14*8
                ld.d    $t3, $sp, 15*8
                ld.d    $t4, $sp, 16*8
                ld.d    $t5, $sp, 17*8
                ld.d    $t6, $sp, 18*8
                ld.d    $t7, $sp, 19*8
                ld.d    $t8, $sp, 20*8

                ld.d    $fp, $sp, 22*8
                ld.d    $s0, $sp, 23*8
                ld.d    $s1, $sp, 24*8
                ld.d    $s2, $sp, 25*8
                ld.d    $s3, $sp, 26*8
                ld.d    $s4, $sp, 27*8
                ld.d    $s5, $sp, 28*8
                ld.d    $s6, $sp, 29*8
                ld.d    $s7, $sp, 30*8
                ld.d    $s8, $sp, 31*8
            
                // restore sp
                ld.d    $sp, $sp, 3*8
                ertn        
        ",
        trapframe_size = const crate::CONTEXT_SIZE,
        trap_handler = sym loongarch64_trap_handler,
        options(noreturn)
    );
}

#[inline]
pub fn set_trap_vector_base() {
    ecfg::set_vs(0);
    eentry::set_eentry(trap_vector_base as usize);
}

fn handle_unaligned(tf: &mut Context) {
    // unsafe { emulate_load_store_insn(tf) }
    error!("address not aligned: {:#x?}", tf);
}

fn handle_breakpoint(era: &mut usize) {
    debug!("Exception(Breakpoint) @ {:#x} ", era);
    *era += 4;
}

fn loongarch64_trap_handler(tf: &mut Context) {
    let estat = estat::read();

    match estat.cause() {
        Trap::Exception(Exception::Breakpoint) => handle_breakpoint(&mut tf.era),
        Trap::Exception(Exception::AddressNotAligned) => handle_unaligned(tf),
        Trap::Interrupt(_) => {
            let irq_num: usize = estat.is().trailing_zeros() as usize;
            info!("irq: {}", irq_num);
        }
        _ => {
            panic!(
                "Unhandled trap {:?} @ {:#x}:\n{:#x?}",
                estat.cause(),
                tf.era,
                tf
            );
        }
    }
}
