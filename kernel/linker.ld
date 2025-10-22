/* Linker script for CosmOS kernel */

OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)

SECTIONS
{
    /* Kernel entry point at 2mb */
    . = 0x200000;

    .text : ALIGN(4K)
    {
        KEEP(*(.text._start))
        *(.text .text.*)
    }

    . = ALIGN(4K);
    .rodata :
    {
        *(.rodata .rodata.*)
    }

    . = ALIGN(4K);
    .data :
    {
        *(.data .data.*)
    }

    . = ALIGN(4K);
    .bss :
    {
        *(COMMON)
        *(.bss .bss.*)
    }

    /DISCARD/ :
    {
        *(.eh_frame_hdr)
        *(.eh_frame)
        *(.note .note.*)
        *(.dynamic)
        *(.dynsym)
        *(.dynstr)
        *(.hash)
        *(.gnu.hash)
        *(.got)
        *(.got.plt)
        *(.plt)
        *(.rela.*)
    }
}
