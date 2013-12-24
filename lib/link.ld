/* 
 * Copyright (C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>.
 *
 * link.ld - User linker script.
 */

ENTRY(_start)

SECTIONS
{
	. = 0x00800000;
	
    .text :
    {
        *(.text)
    }

    .data :
    {
		. = ALIGN(4096);
        *(.data)
        *(.rodata)
        *(.bss)
    }
}
