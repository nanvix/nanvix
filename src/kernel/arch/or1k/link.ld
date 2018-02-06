/*
 * Copyright(C) 2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2017-2018 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * This file is part of Nanvix.
 * 
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 * 
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

OUTPUT_FORMAT("elf32-or1k")
ENTRY(start)

SECTIONS 
{
	EXCEPTIONS = .;

	.exception :
	{
		arch/or1k/exception.o *(.exception)
	}
	
	KSTART = .;

	/* Bootstrap section. */
	.bootstrap :
	{
		arch/or1k/boot.o *(.bootstrap)
	}
	
	. += 0xc0000000;

	/* Kernel code section. */
	.text ALIGN(8192) : AT(ADDR(.text) - 0xc0000000)
	{
		*(.text)
		*(.rodata)
	}
   
	/* Initialized kernel data section. */
   .data ALIGN(8192) : AT(ADDR(.data) - 0xc0000000)
   {
       *(.data)
   }
   
   /* Uninitialized kernel data section. */
   .bss : AT(ADDR(.bss) - 0xc0000000)
   {
       *(.bss)
   }
   
   . =ALIGN(8192);
	
   KDATA_END = .;
   
   /* Discarded. */
   /DISCARD/ :
   {
        *(.comment)
        *(.note)
   }
}
