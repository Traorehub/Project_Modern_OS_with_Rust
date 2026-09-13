OUTPUT_FORMAT("elf64-x86-64")
ENTRY(_start)

SECTIONS
{
    . = 0x200000;

    .text : {
        *(.text.entry)
        reload_cs.o(.text)
        *(.text*)
    }

    .rodata : {
        *(.rodata*)
        *(.srodata*)
    }

    /* Tables de pointeurs resolues a l'edition de liens.
       Si elles ne sont pas nommees ici, lld les place dans des sections
       orphelines que l'image disque ne recopie pas : les pointeurs sont
       alors lus a zero et tout appel indirect saute a l'adresse 0. */
    .data.rel.ro : {
        *(.data.rel.ro*)
    }

    .got : {
        *(.got)
        *(.got.plt)
        *(.igot*)
    }

    .data : {
        *(.data*)
        *(.sdata*)
    }

    .bss : {
        *(.bss*)
        *(.sbss*)
        *(COMMON)
    }

    /DISCARD/ : {
        *(.eh_frame*)
        *(.comment)
        *(.note*)
    }
}
