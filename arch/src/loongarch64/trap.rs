use core::arch::{asm, global_asm};

use loongarch64::register::estat::{self, Exception, Trap};
use loongarch64::register::{
    badv, crmd, ecfg, eentry, pwch, pwcl, stlbps, ticlr, tlbidx, tlbrehi, tlbrentry,
};

use crate::{ArchInterface, TrapType};

use super::Context;

global_asm!(
    r"
        .altmacro
        .equ KSAVE_KSP,  0x30
        .equ KSAVE_CTX,  0x31
        .equ KSAVE_USP,  0x32
        .macro SAVE_REGS
            st.d    $ra, $sp,  1*8
            st.d    $tp, $sp,  2*8
            st.d    $a0, $sp,  4*8
            st.d    $a1, $sp,  5*8
            st.d    $a2, $sp,  6*8
            st.d    $a3, $sp,  7*8
            st.d    $a4, $sp,  8*8
            st.d    $a5, $sp,  9*8
            st.d    $a6, $sp, 10*8
            st.d    $a7, $sp, 11*8
            st.d    $t0, $sp, 12*8
            st.d    $t1, $sp, 13*8
            st.d    $t2, $sp, 14*8
            st.d    $t3, $sp, 15*8
            st.d    $t4, $sp, 16*8
            st.d    $t5, $sp, 17*8
            st.d    $t6, $sp, 18*8
            st.d    $t7, $sp, 19*8
            st.d    $t8, $sp, 20*8
            st.d    $r21,$sp, 21*8
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
            csrrd   $t0, KSAVE_USP
            st.d    $t0, $sp,  3*8

            csrrd	$t0, 0x1
            st.d	$t0, $sp, 8*32  // prmd

            csrrd   $t0, 0x6        
            st.d    $t0, $sp, 8*33  // era
        .endm

        .macro LOAD_REGS
            ld.d    $t0, $sp, 32*8
            csrwr   $t0, 0x1        // Write PRMD(PLV PIE PWE) to prmd

            ld.d    $t0, $sp, 33*8
            csrwr   $t0, 0x6        // Write Exception Address to ERA

            ld.d    $ra, $sp, 1*8
            ld.d    $tp, $sp, 2*8
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
            ld.d    $r21,$sp, 21*8
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
        .endm
    "
);

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
pub unsafe extern "C" fn user_vec() {
    core::arch::asm!(
        "
            csrrd   $sp,  KSAVE_CTX
            SAVE_REGS

            csrrd   $sp,  KSAVE_KSP
            ld.d    $ra,  $sp, 0*8
            ld.d    $tp,  $sp, 1*8
            ld.d    $r21, $sp, 2*8
            ld.d    $s9,  $sp, 3*8
            ld.d    $s0,  $sp, 4*8
            ld.d    $s1,  $sp, 5*8
            ld.d    $s2,  $sp, 6*8
            ld.d    $s3,  $sp, 7*8
            ld.d    $s4,  $sp, 8*8
            ld.d    $s5,  $sp, 9*8
            ld.d    $s6,  $sp, 10*8
            ld.d    $s7,  $sp, 11*8
            ld.d    $s8,  $sp, 12*8
            addi.d  $sp,  $sp, 13*8
            ret

        ",
        options(noreturn)
    );
}

#[naked]
#[no_mangle]
pub extern "C" fn user_restore(context: *mut Context) {
    unsafe {
        asm!(
            r"
                addi.d  $sp,  $sp, -13*8
                st.d    $ra,  $sp, 0*8
                st.d    $tp,  $sp, 1*8
                st.d    $r21, $sp, 2*8
                st.d    $s9,  $sp, 3*8
                st.d    $s0,  $sp, 4*8
                st.d    $s1,  $sp, 5*8
                st.d    $s2,  $sp, 6*8
                st.d    $s3,  $sp, 7*8
                st.d    $s4,  $sp, 8*8
                st.d    $s5,  $sp, 9*8
                st.d    $s6,  $sp, 10*8
                st.d    $s7,  $sp, 11*8
                st.d    $s8,  $sp, 12*8

                csrwr    $sp, KSAVE_KSP   // SAVE kernel_sp to SAVEn(0)
                move     $sp, $a0         // TIPS: csrwr will write the old value to rd
                csrwr    $a0, KSAVE_CTX   // SAVE user context addr to SAVEn(1)

                LOAD_REGS

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
    user_restore(cx);
    match loongarch64_trap_handler(cx) {
        TrapType::UserEnvCall => Some(()),
        _ => None,
    }
}

#[naked]
pub unsafe extern "C" fn trap_vector_base() {
    core::arch::asm!(
        "
            .balign 4096
                // Check whether it was from user privilege.
                csrwr   $sp, KSAVE_USP
                csrrd   $sp, 0x1
                andi    $sp, $sp, 0x3
                bnez    $sp, {user_vec} 
            
                csrrd   $sp, KSAVE_USP
                addi.d  $sp, $sp, -{trapframe_size} // allocate space
            
                // save the registers.

                SAVE_REGS
            
                move    $a0, $sp
                bl      {trap_handler}
            
                // Load registers from sp, include new sp
                LOAD_REGS
                ertn
        ",
        trapframe_size = const crate::CONTEXT_SIZE,
        user_vec = sym user_vec,
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
pub const _PS_16K: usize = 0x0e;
pub const _PS_2M: usize = 0x15;
pub const _PS_1G: usize = 0x1e;

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

fn loongarch64_trap_handler(tf: &mut Context) -> TrapType {
    let estat = estat::read();
    let trap_type = match estat.cause() {
        Trap::Exception(Exception::Breakpoint) => {
            debug!("Exception(Breakpoint) @ {:#x} ", tf.era);
            tf.era += 4;
            TrapType::Breakpoint
        }
        Trap::Exception(Exception::AddressNotAligned) => {
            error!("address not aligned: {:#x?}", tf);
            TrapType::Unknown
        }
        Trap::Interrupt(_) => {
            let irq_num: usize = estat.is().trailing_zeros() as usize;
            match irq_num {
                // TIMER_IRQ
                11 => {
                    ticlr::clear_timer_interrupt();
                    TrapType::Time
                }
                _ => panic!("unknown interrupt: {}", irq_num),
            }
        }
        Trap::Exception(Exception::Syscall) => TrapType::UserEnvCall,
        Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::PageModifyFault) => {
            TrapType::StorePageFault(badv::read().raw())
        }
        Trap::Exception(Exception::LoadPageFault) => TrapType::LoadPageFault(badv::read().raw()),
        _ => {
            panic!(
                "Unhandled trap {:?} @ {:#x} BADV: {:#x}:\n{:#x?}",
                estat.cause(),
                tf.era,
                badv::read().raw(),
                tf
            );
        }
    };
    ArchInterface::kernel_interrupt(tf, trap_type);
    trap_type
}
