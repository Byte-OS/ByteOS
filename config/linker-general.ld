OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0xffffffc080200000;
__ENTRY_ADDR = 0x80200000;

SECTIONS
{
    /* Load the kernel at this address: "." means the current address */
    . = BASE_ADDRESS;
    start = .;

    .text ALIGN(4K): AT(__ENTRY_ADDR) {
        stext = .;
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
        sdata = .;
        *(.data .data.*)
        *(.sdata .sdata.*)
        edata = .;
    }

    .bss ALIGN(4K): {
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        ebss = .;
    }

    PROVIDE(end = .);
}