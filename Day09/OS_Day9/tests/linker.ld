ENTRY(_start)

SECTIONS {
    . = 0x8000;
    .text.entry : { *(.text.entry) }
    .text       : { *(.text) }
    .rodata     : { *(.rodata) }
    .data       : { *(.data) }
    .bss        : { *(.bss) }
}
