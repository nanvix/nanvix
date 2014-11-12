/* 
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>.
 *
 * link.ld - Kernel linker script.
 */

OUTPUT_FORMAT("elf32-i386")
ENTRY(start)

SECTIONS 
{
	. = 0x100000;
	
	KSTART = .;
	
	/* Bootstrap section. */
	.bootstrap :
	{
		arch/x86/boot.o *(.bootstrap)
	}
	
	. += 0xc0000000;

	/* Kernel code section. */
	.text ALIGN(4096) : AT(ADDR(.text) - 0xc0000000)
   {
       *(.text)
       *(.rodata)
   }
   
   /* Initialized kernel data section. */
   .data ALIGN(4096) : AT(ADDR(.data) - 0xc0000000)
   {
       *(.data)
   }
   
   /* Uninitialized kernel data section. */
   .bss : AT(ADDR(.bss) - 0xc0000000)
   {
       *(.bss)
   }
   
   . =ALIGN(4096);
	
   KDATA_END = .;
   
   /* Discarded. */
   /DISCARD/ :
   {
        *(.comment)
        *(.note)
   }
}
