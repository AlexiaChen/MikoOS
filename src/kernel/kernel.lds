OUTPUT_FORMAT("elf64-x86-64","elf64-x86-64","elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)
SECTIONS
{

	. = 0xffff800000000000 + 0x100000;
	.text :
	{
		_text = .;
		*(.text)

		_endtext = .;
	}
	. = ALIGN(8);
	.data :
	{
		_data = .;
		*(.data)
		
		_enddata = .;
	}
	.bss :
	{
		_bss = .;
		*(.bss)
		_endbss = .;
	}

	_end = .;
}