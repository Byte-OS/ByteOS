OUTPUT_ARCH(x86_64)
ENTRY(_start)

/* BASE_ADDRESS = 0xffffffc000020000; */
BASE_ADDRESS = 0xffffff8000200000;

__ENTRY_ADDR = 0x200000;

SECTIONS
{
    /* Load the kernel at this address: "." means the current address */
    . = BASE_ADDRESS;
    start = .;
    _skernel = .;

    .text ALIGN(4K): AT(__ENTRY_ADDR) {
        stext = .;
        *(.text.boot)
        *(.text.entry)
        *(.text .text.*)
        etext = .;
    }

    .sigtrx ALIGN(4K): {
        *(.sigtrx .sigtrx.*)
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

    .bss ALIGN(4K): {
        *(.bss.stack)
        _sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        _ebss = .;
    }

    PROVIDE(end = .);
}