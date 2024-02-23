OUTPUT_ARCH(%ARCH%)
ENTRY(_start)

BASE_ADDRESS = %KERNEL_BASE%;

SECTIONS
{
    /* Load the kernel at this address: "." means the current address */
    . = BASE_ADDRESS;
    start = .;
    _skernel = .;

    .text ALIGN(4K): {
        stext = .;
        *(.text.entry)
        *(.text .text.*)
        etext = .;
    }

    .rodata ALIGN(4K): {
        srodata = .;
        *(.rodata .rodata.*)
        . = ALIGN(4K);
        erodata = .;
    }

    .data ALIGN(4K): {
        . = ALIGN(4K);
        *(.data.prepage .data.prepage.*)
        . = ALIGN(4K);
        _sdata = .;
        *(.data .data.*)
        *(.sdata .sdata.*)
        _edata = .;
    }

    .sigtrx ALIGN(4K): {
        *(.sigtrx .sigtrx.*)
    }

    _load_end = .;

    . = ALIGN(4K);
    _percpu_start = .;
    .percpu 0x0 : AT(_percpu_start) {
        _percpu_load_start = .;
        *(.percpu .percpu.*)
        _percpu_load_end = .;
        . = ALIGN(64);
        _percpu_size_aligned = .;

        . = _percpu_load_start + _percpu_size_aligned * %SMP%;
    }
    . = _percpu_start + SIZEOF(.percpu);
    _percpu_end = .;

    .bss ALIGN(4K): {
        *(.bss.stack)
        _sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        _ebss = .;
    }

    PROVIDE(end = .);
    /DISCARD/ : {
        *(.comment) *(.gnu*) *(.note*) *(.eh_frame*)
    }
}