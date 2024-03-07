.equ NUM_INT, 256

.altmacro
.macro DEF_HANDLER, i
.Ltrap_handler_\i:
.if \i == 8 || (\i >= 10 && \i <= 14) || \i == 17
    # error code pushed by CPU
    push    \i          # interrupt vector
    jmp     .Ltrap_common
.else
    push    0           # fill in error code in TrapFrame
    push    \i          # interrupt vector
    jmp     .Ltrap_common
.endif
.endm

.macro DEF_TABLE_ENTRY, i
    .quad .Ltrap_handler_\i
.endm

.section .text
.code64
_trap_handlers:
.set i, 0
.rept NUM_INT
    DEF_HANDLER %i
    .set i, i + 1
.endr

.Ltrap_common:
    test    byte ptr [rsp + 3 * 8], 3   # swap GS if it comes from user space
    jnz      .Ltrap_from_user
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
    call    x86_trap_handler

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

    add     rsp, 16                     # pop vector, error_code
    iretq

.Ltrap_from_user:
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

    mov    rsp, gs:[offset __PERCPU_KERNEL_RSP]  // kernel rsp

    pop rcx
    mov ss, rcx
    pop rax
    mov cs, rax
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp

    ret

.section .rodata
.global trap_handler_table
trap_handler_table:
.set i, 0
.rept NUM_INT
    DEF_TABLE_ENTRY %i
    .set i, i + 1
.endr
