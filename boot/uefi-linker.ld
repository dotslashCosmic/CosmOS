/* PE32+ COFF Linker Script for UEFI Bootloader */

OUTPUT_FORMAT(pei-x86-64)
ENTRY(efi_main)

SECTIONS {
    /* Start at address 0 */
    . = 0;
    
    /* Image base for PE/COFF */
    ImageBase = .;
    
    /* Code section */
    .text : {
        *(.text .text.*)
    }
    
    /* Read-only data section */
    .rodata : {
        *(.rodata .rodata.*)
    }
    
    /* Data section */
    .data : {
        *(.data .data.*)
    }
    
    /* BSS section */
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }
    
    /* Discard sections */
    /DISCARD/ : {
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.note.*)
        *(.comment)
        *(.debug*)
    }
}
