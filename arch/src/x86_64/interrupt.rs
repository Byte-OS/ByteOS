use core::arch::{asm, global_asm};
use core::mem::size_of;

use bitflags::bitflags;
use x86_64::registers::model_specific::{Efer, EferFlags, KernelGsBase, LStar, SFMask, Star};
use x86_64::registers::rflags::RFlags;
use x86_64::VirtAddr;

use x86::{controlregs::cr2, irq::*};

use crate::consts::TRAPFRAME_SIZE;
use crate::currrent_arch::gdt::set_tss_kernel_sp;
use crate::{currrent_arch::gdt::GdtStruct, TrapFrame, TrapType};
use crate::{ArchInterface, SYSCALL_VECTOR};

use super::apic::vectors::APIC_TIMER_VECTOR;
use super::context::FxsaveArea;
use super::time::ticks_to_nanos;

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

global_asm!(include_str!("trap.S"));

#[no_mangle]
#[percpu::def_percpu]
static USER_RSP: usize = 0;

#[no_mangle]
#[percpu::def_percpu]
static KERNEL_RSP: usize = 0;

#[no_mangle]
#[percpu::def_percpu]
static USER_CONTEXT: usize = 0;

bitflags! {
    // https://wiki.osdev.org/Exceptions#Page_Fault
    #[derive(Debug)]
    struct PageFaultFlags: u32 {
        const P = 1;
        const W = 1 << 1;
        const U = 1 << 2;
        const R = 1 << 3;
        const I = 1 << 4;
        const PK = 1 << 5;
        const SS = 1 << 6;
        const SGX = 1 << 15;
    }
}

// 内核中断回调
#[no_mangle]
fn kernel_callback(context: &mut TrapFrame) {
    let trap_type = match context.vector as u8 {
        PAGE_FAULT_VECTOR => {
            let pflags = PageFaultFlags::from_bits_truncate(context.rflags as _);
            // debug!("flags: {:#x?} cx_ref: {:#x?}", pflags, context);
            if pflags.contains(PageFaultFlags::I) {
                TrapType::InstructionPageFault(unsafe { cr2() })
            } else if pflags.contains(PageFaultFlags::W) {
                TrapType::StorePageFault(unsafe { cr2() })
            } else {
                TrapType::LoadPageFault(unsafe { cr2() })
            }
        }
        BREAKPOINT_VECTOR => {
            debug!("#BP @ {:#x} ", context.rip);
            TrapType::Breakpoint
        }
        GENERAL_PROTECTION_FAULT_VECTOR => {
            panic!(
                "#GP @ {:#x}, fault_vaddr={:#x} error_code={:#x}:\n{:#x?}",
                context.rip,
                unsafe { cr2() },
                context.error_code,
                context
            );
        }
        APIC_TIMER_VECTOR => TrapType::Time,
        // IRQ_VECTOR_START..=IRQ_VECTOR_END => crate::trap::handle_irq_extern(tf.vector as _),
        _ => {
            panic!(
                "Unhandled exception {} (error_code = {:#x}) @ {:#x}:\n{:#x?}",
                context.vector, context.error_code, context.rip, context
            );
        }
    };
    ArchInterface::kernel_interrupt(context, trap_type);
    unsafe { super::apic::local_apic().end_of_interrupt() };
}

#[naked]
#[no_mangle]
pub unsafe extern "C" fn kernelvec() {
    asm!(
        r"
            sub     rsp, 16                     # push fs_base, gs_base

            push    r15
            push    r14
            push    r13
            push    r12
            push    r11
            push    r10
            push    r9
            push    r8
            push    rdi
            push    rsi
            push    rbp
            push    rbx
            push    rdx
            push    rcx
            push    rax

            mov     rdi, rsp
            call    {trap_handler}

            pop     rax
            pop     rcx
            pop     rdx
            pop     rbx
            pop     rbp
            pop     rsi
            pop     rdi
            pop     r8
            pop     r9
            pop     r10
            pop     r11
            pop     r12
            pop     r13
            pop     r14
            pop     r15

            add     rsp, 32                     # pop fs_base, gs_base, vector, error_code
            iretq
        ",
        trap_handler = sym kernel_callback,
        options(noreturn)
    )
}

#[naked]
#[no_mangle]
pub unsafe extern "C" fn uservec() {
    asm!(
        r"
            sub     rsp, 16

            push    r15
            push    r14
            push    r13
            push    r12
            push    r11
            push    r10
            push    r9
            push    r8
            push    rdi
            push    rsi
            push    rbp
            push    rbx
            push    rdx
            push    rcx
            push    rax

            swapgs

            mov     rdi, rsp
            mov    rsp, gs:[offset __PERCPU_KERNEL_RSP]  // kernel rsp

            pop r15
            pop r14
            pop r13
            pop r12
            pop rbx
            pop rbp
            pop rax

            mov ecx, 0xC0000100
            mov rdx, rax
            shr rdx, 32
            wrmsr                   # pop fsbase

            ret
        ",
        options(noreturn)
    );
}

#[naked]
#[no_mangle]
pub extern "C" fn user_restore(context: *mut TrapFrame) {
    unsafe {
        asm!(
            // Save callee saved registers and cs and others.
            r"
                mov ecx, 0xC0000100
                rdmsr
                shl rdx, 32
                or  rax, rdx
                push rax                # push fsbase

                push rbp
                push rbx
                push r12
                push r13
                push r14
                push r15

                mov gs:[offset __PERCPU_KERNEL_RSP], rsp
            ",
            // Write fs_base and gs_base
            "
                mov ecx, 0xC0000100
                mov edx, [rdi + 15*8+4]
                mov eax, [rdi + 15*8]
                wrmsr                   # pop fsbase
                mov ecx, 0xC0000102
                mov edx, [rdi + 16*8+4]
                mov eax, [rdi + 16*8]
                wrmsr                   # pop gsbase to kernel_gsbase
            ",
            // push fs_base
            "
                mov     rsp, rdi
                pop     rax
                pop     rcx
                pop     rdx
                pop     rbx
                pop     rbp
                pop     rsi
                pop     rdi
                pop     r8
                pop     r9
                pop     r10
                pop     r11
                pop     r12
                pop     r13
                pop     r14
                pop     r15

                add     rsp, 32         # pop fs_base,gs_base,vector,error_code
                cmp DWORD PTR [rsp - 8*2], {syscall_vector}
                je      {sysretq}
                
                swapgs
                iretq
            ",
            syscall_vector = const SYSCALL_VECTOR,
            sysretq = sym sysretq,
            options(noreturn)
        )
    }
}

#[naked]
unsafe extern "C" fn sysretq() {
    asm!(
        "
            pop rcx
            add rsp, 8
            pop r11
            pop rsp
            swapgs

            sysretq
        ",
        options(noreturn)
    )
}

pub fn init_syscall() {
    LStar::write(VirtAddr::new(syscall_entry as usize as _));
    Star::write(
        GdtStruct::UCODE64_SELECTOR,
        GdtStruct::UDATA_SELECTOR,
        GdtStruct::KCODE64_SELECTOR,
        GdtStruct::KDATA_SELECTOR,
    )
    .unwrap();
    SFMask::write(
        RFlags::TRAP_FLAG
            | RFlags::INTERRUPT_FLAG
            | RFlags::DIRECTION_FLAG
            | RFlags::IOPL_LOW
            | RFlags::IOPL_HIGH
            | RFlags::NESTED_TASK
            | RFlags::ALIGNMENT_CHECK,
    ); // TF | IF | DF | IOPL | AC | NT (0x47700)
    unsafe {
        Efer::update(|efer| *efer |= EferFlags::SYSTEM_CALL_EXTENSIONS);
    }
    KernelGsBase::write(VirtAddr::new(0));
}

#[naked]
unsafe extern "C" fn syscall_entry() {
    asm!(
        r"
            swapgs
            mov     gs:[offset __PERCPU_USER_RSP], rsp
            mov     rsp, gs:[offset __PERCPU_USER_CONTEXT]
        
            sub     rsp, 8                          // skip user_ss
            push    gs:[offset __PERCPU_USER_RSP]   // user_rsp
            push    r11                             // rflags
            mov     [rsp - 2 * 8], rcx              // rip
            mov     r11, {syscall_vector}
            mov     [rsp - 4 * 8], r11              // vector
            sub     rsp, 6 * 8                      // skip until general registers

            push    r15
            push    r14
            push    r13
            push    r12
            push    r11
            push    r10
            push    r9
            push    r8
            push    rdi
            push    rsi
            push    rbp
            push    rbx
            push    rdx
            push    rcx
            push    rax

            mov ecx, 0xC0000100
            rdmsr
            mov [rsp + 15*8+4], edx
            mov [rsp + 15*8], eax   # push fabase

            mov ecx, 0xC0000102
            rdmsr
            mov [rsp + 16*8+4], edx
            mov [rsp + 16*8], eax   # push gs_base
        
            mov    rsp, gs:[offset __PERCPU_KERNEL_RSP]  // kernel rsp
            pop r15
            pop r14
            pop r13
            pop r12
            pop rbx
            pop rbp
            pop rax

            mov ecx, 0xC0000100
            mov rdx, rax
            shr rdx, 32
            wrmsr                   # pop fsbase
            ret
        ",
        syscall_vector = const SYSCALL_VECTOR,
        options(noreturn)
    )
}

/// Return Some(()) if it was interrupt by syscall, otherwise None.
pub fn run_user_task(context: &mut TrapFrame) -> Option<()> {
    // TODO: set tss kernel sp just once, before task run.
    let cx_general_top =
        context as *mut TrapFrame as usize + TRAPFRAME_SIZE - size_of::<FxsaveArea>();
    set_tss_kernel_sp(cx_general_top);
    USER_CONTEXT.write_current(cx_general_top);
    context.fx_area.restore();
    user_restore(context);
    context.fx_area.save();

    match context.vector {
        SYSCALL_VECTOR => {
            ArchInterface::kernel_interrupt(context, TrapType::UserEnvCall);
            Some(())
        },
        _ => {
            kernel_callback(context);
            None
        }
    }
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
    // Test break point.
    unsafe { core::arch::asm!("int 3") }
}

pub fn time_to_usec(ticks: usize) -> usize {
    (ticks_to_nanos(ticks as _) / 1000) as _
}

pub fn get_time() -> usize {
    unsafe { core::arch::x86_64::_rdtsc() as _ }
}
