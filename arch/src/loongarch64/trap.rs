use core::arch::asm;

use loongarch64::register::{
    badv, crmd, ecfg, eentry,
    estat::{self, Exception, Trap},
    pgdh, pgdl, pwch, pwcl, stlbps, tlbidx, tlbrehi, tlbrentry,
};

use super::Context;

// 设置中断
pub fn init_interrupt() {
    unsafe {
        core::arch::asm!("break 2");
    }
    tlb_init(tlb_fill as _);
    info!("tlb_fill addr: {:#x}", tlb_fill as usize);
    let pwcl = pwcl::read().raw();
    info!("PTEWitdth: {}", pwcl >> 30);
    info!(
        "PTBase: {}, witdh: {}",
        (pwcl >> 0) & 0x1f,
        (pwcl >> 5) & 0x1f
    );
    info!(
        "Dir1Base: {}, witdh: {}",
        (pwcl >> 10) & 0x1f,
        (pwcl >> 15) & 0x1f
    );
    info!(
        "Dir2Base: {}, witdh: {}",
        (pwcl >> 20) & 0x1f,
        (pwcl >> 25) & 0x1f
    );
    let pwch = pwch::read().raw();
    info!(
        "Dir3Base: {}, witdh: {}",
        (pwch >> 0) & 0x3f,
        (pwch >> 6) & 0x3f
    );
    info!(
        "Dir4Base: {}, witdh: {}",
        (pwch >> 12) & 0x3f,
        (pwch >> 18) & 0x3f
    );

    enable_irq();
}

#[naked]
#[no_mangle]
pub extern "C" fn user_restore(context: *mut Context) {
    unsafe {
        asm!(
            r"
                addi.d  $sp, $sp, -14*8
                st.d    $r1,  $sp, 0*8
                st.d    $r2,  $sp, 1*8
                st.d    $r3,  $sp, 2*8
                st.d    $r21, $sp, 3*8
                st.d    $r22, $sp, 4*8
                st.d    $r23, $sp, 5*8
                st.d    $r24, $sp, 6*8
                st.d    $r25, $sp, 7*8
                st.d    $r26, $sp, 8*8
                st.d    $r27, $sp, 9*8
                st.d    $r28, $sp, 10*8
                st.d    $r29, $sp, 11*8
                st.d    $r30, $sp, 12*8
                st.d    $r31, $sp, 13*8
                csrwr    $sp, 0x30      // SAVE kernel_sp to SAVEn(0)
                move     $a1, $a0       // TIPS: csrwr will write the old value to rd
                csrwr    $a1, 0x31      // SAVE user context addr to SAVEn(1)

                ld.d    $t0, $a0, 33*8
                csrwr   $t0, 0x6        // Write Exception Address to ERA

                ld.d    $ra, $a0, 1*8
                ld.d    $tp, $a0, 2*8
                ld.d    $sp, $a0, 3*8
                ld.d    $a1, $a0, 5*8
                ld.d    $a2, $a0, 6*8
                ld.d    $a3, $a0, 7*8
                ld.d    $a4, $a0, 8*8
                ld.d    $a5, $a0, 9*8
                ld.d    $a6, $a0, 10*8
                ld.d    $a7, $a0, 11*8
                ld.d    $t0, $a0, 12*8
                ld.d    $t1, $a0, 13*8
                ld.d    $t2, $a0, 14*8
                ld.d    $t3, $a0, 15*8
                ld.d    $t4, $a0, 16*8
                ld.d    $t5, $a0, 17*8
                ld.d    $t6, $a0, 18*8
                ld.d    $t7, $a0, 19*8
                ld.d    $t8, $a0, 20*8
                ld.d    $r21,$a0, 21*8
                ld.d    $fp, $a0, 22*8
                ld.d    $s0, $a0, 23*8
                ld.d    $s1, $a0, 24*8
                ld.d    $s2, $a0, 25*8
                ld.d    $s3, $a0, 26*8
                ld.d    $s4, $a0, 27*8
                ld.d    $s5, $a0, 28*8
                ld.d    $s6, $a0, 29*8
                ld.d    $s7, $a0, 30*8
                ld.d    $s8, $a0, 31*8
            
                // restore sp
                ld.d    $a0, $a0, 4*8
                ertn
            ",
            options(noreturn)
        )
    }
}

#[allow(dead_code)]
#[inline(always)]
pub fn enable_irq() {
    crmd::set_ie(true);
}

#[inline(always)]
pub fn enable_external_irq() {
    // unsafe {
    //     sie::set_sext();
    // }
}

pub fn run_user_task(cx: &mut Context) -> Option<()> {
    info!("run user task: {:#x?}", cx);
    user_restore(cx);
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

#[naked]
pub unsafe extern "C" fn tlb_fill() {
    core::arch::asm!(
        "
        .equ LA_CSR_PGDL,          0x19    /* Page table base address when VA[47] = 0 */
        .equ LA_CSR_PGDH,          0x1a    /* Page table base address when VA[47] = 1 */
        .equ LA_CSR_PGD,           0x1b    /* Page table base */
        .equ LA_CSR_TLBRENTRY,     0x88    /* TLB refill exception entry */
        .equ LA_CSR_TLBRBADV,      0x89    /* TLB refill badvaddr */
        .equ LA_CSR_TLBRERA,       0x8a    /* TLB refill ERA */
        .equ LA_CSR_TLBRSAVE,      0x8b    /* KScratch for TLB refill exception */
        .equ LA_CSR_TLBRELO0,      0x8c    /* TLB refill entrylo0 */
        .equ LA_CSR_TLBRELO1,      0x8d    /* TLB refill entrylo1 */
        .equ LA_CSR_TLBREHI,       0x8e    /* TLB refill entryhi */
        .balign 4096
            csrwr   $t0, LA_CSR_TLBRSAVE
            csrrd   $t0, LA_CSR_PGD
            lddir   $t0, $t0, 3
            lddir   $t0, $t0, 1
            ldpte   $t0, 0
            ldpte   $t0, 1
            tlbfill
            csrrd   $t0, LA_CSR_TLBRSAVE
            ertn
        ",
        options(noreturn)
    );
}

#[inline]
pub fn set_tlb_refill(tlbrentry: usize) {
    tlbrentry::set_tlbrentry(tlbrentry & 0xFFFF_FFFF_FFFF);
}

pub const PS_4K: usize = 0x0c;
pub const PS_16K: usize = 0x0e;
pub const PS_2M: usize = 0x15;
pub const PS_1G: usize = 0x1e;

pub const PAGE_SIZE_SHIFT: usize = 12;

pub fn tlb_init(tlbrentry: usize) {
    // // setup PWCTL
    // unsafe {
    // asm!(
    //     "li.d     $r21,  0x4d52c",     // (9 << 15) | (21 << 10) | (9 << 5) | 12
    //     "csrwr    $r21,  0x1c",        // LOONGARCH_CSR_PWCTL0
    //     "li.d     $r21,  0x25e",       // (9 << 6)  | 30
    //     "csrwr    $r21,  0x1d",         //LOONGARCH_CSR_PWCTL1
    //     )
    // }

    tlbidx::set_ps(PS_4K);
    stlbps::set_ps(PS_4K);
    tlbrehi::set_ps(PS_4K);

    // set hardware
    pwcl::set_pte_width(8); // 64-bits
    pwcl::set_ptbase(PAGE_SIZE_SHIFT);
    pwcl::set_ptwidth(PAGE_SIZE_SHIFT - 3);

    pwcl::set_dir1_base(PAGE_SIZE_SHIFT + PAGE_SIZE_SHIFT - 3);
    pwcl::set_dir1_width(PAGE_SIZE_SHIFT - 3);

    pwch::set_dir3_base(PAGE_SIZE_SHIFT + PAGE_SIZE_SHIFT - 3 + PAGE_SIZE_SHIFT - 3);
    pwch::set_dir3_width(PAGE_SIZE_SHIFT - 3);

    set_tlb_refill(tlbrentry);
    // pgdl::set_base(kernel_pgd_base);
    // pgdh::set_base(kernel_pgd_base);
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
                "Unhandled trap {:?} @ {:#x} BADV: {:#x}:\n{:#x?}",
                estat.cause(),
                tf.era,
                badv::read().raw(),
                tf
            );
        }
    }
}
