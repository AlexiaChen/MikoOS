OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)
SECTIONS
{
    . = 0xffff800000100000;
    .text : {
        _text = .;
        KEEP(*(.text.boot))
        *(.text .text.* .ltext .ltext.*)
        _endtext = .;
    }
    .rodata ALIGN(4096) : { *(.rodata .rodata.* .lrodata .lrodata.*) }
    .data ALIGN(4096) : {
        _data = .;
        *(.data .data.* .ldata .ldata.*)
        *(.got .got.*)
        _enddata = .;
    }
    .bss ALIGN(4096) (NOLOAD) : {
        _bss = .;
        *(.bss .bss.* .lbss .lbss.*)
        *(COMMON)
        _endbss = .;
    }
    _end = ALIGN(4096);
    /DISCARD/ : { *(.eh_frame*) *(.comment) *(.note*) }
    /* 初始直映 RAM 只有 10 MiB，10 MiB 起是显存窗口。 */
    ASSERT(_end < 0xffff800000a00000, "kernel exceeds bootstrap RAM mapping")
}
