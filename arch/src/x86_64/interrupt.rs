use core::arch::{asm, global_asm};

use x86_64::registers::model_specific::{Efer, EferFlags, KernelGsBase, LStar, SFMask, Star};
use x86_64::registers::rflags::RFlags;
use x86_64::VirtAddr;

use x86::{controlregs::cr2, irq::*};

use crate::{x86_64::gdt::GdtStruct, Context, TrapType};

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

#[cfg(target_arch = "x86_64")]
#[no_mangle]
#[percpu::def_percpu]
static USER_RSP: usize = 0;

#[cfg(target_arch = "x86_64")]
#[no_mangle]
#[percpu::def_percpu]
static KERNEL_RSP: usize = 0;

// 内核中断回调
fn kernel_callback(context: &mut Context) -> usize {
    match context.vector as u8 {
        PAGE_FAULT_VECTOR => {
            panic!(
                "Kernel #PF @ {:#x}, fault_vaddr={:#x}, error_code={:#x}:\n{:#x?}",
                context.rip,
                unsafe { cr2() },
                context.error_code,
                context,
            );
        }
        BREAKPOINT_VECTOR => debug!("#BP @ {:#x} ", context.rip),
        GENERAL_PROTECTION_FAULT_VECTOR => {
            panic!(
                "#GP @ {:#x}, error_code={:#x}:\n{:#x?}",
                context.rip, context.error_code, context
            );
        }
        // IRQ_VECTOR_START..=IRQ_VECTOR_END => crate::trap::handle_irq_extern(tf.vector as _),
        _ => {
            panic!(
                "Unhandled exception {} (error_code = {:#x}) @ {:#x}:\n{:#x?}",
                context.vector, context.error_code, context.rip, context
            );
        }
    }
    context as *const Context as usize
}

pub fn trap_pre_handle(_context: &mut Context) -> TrapType {
    todo!("todo trap_pre_handle")
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
pub extern "C" fn user_restore(context: *mut Context) {
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
                
                swapgs
                iretq
            ",
            options(noreturn)
        )
    }
}

#[naked]
#[no_mangle]
pub unsafe extern "C" fn uservec() {
    asm!(
        r"
            mov ecx, 0xC0000100
            rdmsr
            mov [rsp + 18*8+4], edx
            mov [rsp + 18*8], eax   # push fabase
        ",
        options(noreturn)
    );
}

#[no_mangle]
fn x86_syscall_handler(tf: &mut Context) {
    debug!("syscall: {:#x?}", tf);
    panic!("syscall_error")
    // tf.rax = handle_syscall(tf.get_syscall_num(), tf.get_syscall_args()) as u64;
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
            mov     rsp, gs:[offset __PERCPU_KERNEL_RSP]
        
            sub     rsp, 8                      // skip user_ss
            push    gs:[offset __PERCPU_USER_RSP]  // user_rsp
            push    r11                         // rflags
            mov     [rsp - 2 * 8], rcx          // rip
            sub     rsp, 6 * 8                  // skip until general registers

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
            call    x86_syscall_handler
        
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
        
            add     rsp, 7 * 8
            mov     rcx, [rsp - 5 * 8]  // rip
            mov     r11, [rsp - 3 * 8]  // rflags
            mov     rsp, [rsp - 2 * 8]  // user_rsp
        
            swapgs
            sysretq
        ",
        options(noreturn)
    )
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
