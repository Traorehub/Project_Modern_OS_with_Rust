ENTRY(_start)

SECTIONS {
    . = 0x8000;
    .text.entry : { *(.text.entry) }
    .text       : { *(.text .text.*) }
    .rodata     : { *(.rodata .rodata.*) }
    .data       : { *(.data .data.*) }
    .bss        : { *(.bss .bss.*) }
}
