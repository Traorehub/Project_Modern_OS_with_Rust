ENTRY(_start)

SECTIONS {
    . = 0x8000;                     /* Entry point for bootloader */

    .text.entry : { *(.text.entry) }   /* _start must be at 0x8000 */
    .text       : { *(.text) }
    .rodata     : { *(.rodata) }
    .data       : { *(.data) }
    .bss        : { *(.bss) }
}
