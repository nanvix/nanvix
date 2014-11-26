/* 
 * Copyright (C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>.
 *
 * link.ld - User linker script.
 */

ENTRY(_start)

SECTIONS
{
	. = 0x00800000;
	
    .text ALIGN(4194304):
    {
        *(.text)
    }
	. = 0x10000000;

    .data ALIGN(4194304):
    {
        *(.data)
        *(.rodata)
        *(.bss)
    }
}
