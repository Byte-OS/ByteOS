.macro INVALID_EXCP, kind, source
.p2align 7
    msr daifset, #2
    sub     sp, sp, 35 * 8
    stp     x0, x1, [sp]
    stp     x2, x3, [sp, 2 * 8]
    stp     x4, x5, [sp, 4 * 8]
    stp     x6, x7, [sp, 6 * 8]
    stp     x8, x9, [sp, 8 * 8]
    stp     x10, x11, [sp, 10 * 8]
    stp     x12, x13, [sp, 12 * 8]
    stp     x14, x15, [sp, 14 * 8]
    stp     x16, x17, [sp, 16 * 8]
    stp     x18, x19, [sp, 18 * 8]
    stp     x20, x21, [sp, 20 * 8]
    stp     x22, x23, [sp, 22 * 8]
    stp     x24, x25, [sp, 24 * 8]
    stp     x26, x27, [sp, 26 * 8]
    stp     x28, x29, [sp, 28 * 8]

    mrs     x9, sp_el0
    mrs     x10, elr_el1
    mrs     x11, spsr_el1
    mrs     x12, tpidr_el0
    stp     x30, x9, [sp, 30 * 8]
    stp     x10, x11, [sp, 32 * 8]
    str     x12, [sp, 34 * 8]

    mov     x0, sp
    mov     x1, \kind
    mov     x2, \source
    bl      handle_exception
    b       .Lexception_return
.endm

.macro USER_TRAP, kind
.p2align 7
    msr daifset, #2
    str     x0, [sp, 17 * 8]
    ldr     x0, [sp, 16 * 8]

    stp      x0,  x1, [x0]
    stp      x2,  x3, [x0, 2 * 8]
    stp      x4,  x5, [x0, 4 * 8]
    stp      x6,  x7, [x0, 6 * 8]
    stp      x8,  x9, [x0, 8 * 8]
    stp     x10, x11, [x0, 10 * 8]
    stp     x12, x13, [x0, 12 * 8]
    stp     x14, x15, [x0, 14 * 8]
    stp     x16, x17, [x0, 16 * 8]
    stp     x18, x19, [x0, 18 * 8]
    stp     x20, x21, [x0, 20 * 8]
    stp     x22, x23, [x0, 22 * 8]
    stp     x24, x25, [x0, 24 * 8]
    stp     x26, x27, [x0, 26 * 8]
    stp     x28, x29, [x0, 28 * 8]

    mrs     x9, sp_el0
    mrs     x10, elr_el1
    mrs     x11, spsr_el1
    mrs     x12, tpidr_el0
    stp     x30, x9, [x0, 30 * 8]
    stp     x10, x11, [x0, 32 * 8]
    str     x12, [x0, 34 * 8]

    ldr     x1, [sp, 17 * 8]
    str     x1, [x0]
    
    ldp     x8, x16, [sp]
    ldp     x17, x18, [sp, 2 * 8]
    ldp     x19, x20, [sp, 4 * 8]
    ldp     x21, x22, [sp, 6 * 8]
    ldp     x23, x24, [sp, 8 * 8]
    ldp     x25, x26, [sp, 10 * 8]
    ldp     x27, x28, [sp, 12 * 8]
    ldp     x29, x30, [sp, 14 * 8]
    str     x0, [sp, 16 * 8]


    mov     x0, sp
    mov     x1, \kind
    add     sp, sp, 18 * 8
    ret
.endm

.section .text
.p2align 12
.global exception_vector_base
exception_vector_base:
    // current EL, with SP_EL0
    INVALID_EXCP 0 0
    INVALID_EXCP 1 0
    INVALID_EXCP 2 0
    INVALID_EXCP 3 0

    // current EL, with SP_ELx
    INVALID_EXCP 0 1
    INVALID_EXCP 1 1
    INVALID_EXCP 2 1
    INVALID_EXCP 3 1

    // lower EL, aarch64
    USER_TRAP 0
    USER_TRAP 1
    USER_TRAP 2
    USER_TRAP 3

    // lower EL, aarch32
    INVALID_EXCP 0 3
    INVALID_EXCP 1 3
    INVALID_EXCP 2 3
    INVALID_EXCP 3 3

.Lexception_return:
    ldr     x12, [sp, 34 * 8]
    ldp     x10, x11, [sp, 32 * 8]
    ldp     x30, x9, [sp, 30 * 8]
    msr     sp_el0, x9
    msr     elr_el1, x10
    msr     spsr_el1, x11
    msr     tpidr_el0, x12

    ldp     x28, x29, [sp, 28 * 8]
    ldp     x26, x27, [sp, 26 * 8]
    ldp     x24, x25, [sp, 24 * 8]
    ldp     x22, x23, [sp, 22 * 8]
    ldp     x20, x21, [sp, 20 * 8]
    ldp     x18, x19, [sp, 18 * 8]
    ldp     x16, x17, [sp, 16 * 8]
    ldp     x14, x15, [sp, 14 * 8]
    ldp     x12, x13, [sp, 12 * 8]
    ldp     x10, x11, [sp, 10 * 8]
    ldp     x8, x9, [sp, 8 * 8]
    ldp     x6, x7, [sp, 6 * 8]
    ldp     x4, x5, [sp, 4 * 8]
    ldp     x2, x3, [sp, 2 * 8]
    ldp     x0, x1, [sp]
    add     sp, sp, 35 * 8
    eret
