/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

OUTPUT_FORMAT("elf64-littleaarch64")
OUTPUT_ARCH(aarch64)
ENTRY(_do_start)

/*
 * Page Size (4 KB)
 */
PAGE_SIZE = 0x1000;

/*
 * Base Address
 */
BASE_ADDR = 0x40000000;

SECTIONS
{
	. = BASE_ADDR;

	/* Text section. */
	.text : ALIGN(PAGE_SIZE)
	{
		*(.crt0*)
		*(.text*)
	}

	/* Initialized data section. */
	.data : ALIGN(PAGE_SIZE)
	{
		*(.data*)
	}

	/* Dynamic section. */
	.dynamic : ALIGN(8)
	{
		PROVIDE(__dynamic_start = .);
		*(.dynamic)
		PROVIDE(__dynamic_end = .);
	}

	/* Constructor / destructor arrays. */
	.preinit_array : ALIGN(8)
	{
		PROVIDE(__preinit_array_start = .);
		KEEP(*(.preinit_array))
		PROVIDE(__preinit_array_end = .);
	}

	.init_array : ALIGN(8)
	{
		PROVIDE(__init_array_start = .);
		KEEP(*(SORT_BY_INIT_PRIORITY(.init_array.*)))
		KEEP(*(.init_array))
		PROVIDE(__init_array_end = .);
	}

	.fini_array : ALIGN(8)
	{
		PROVIDE(__fini_array_start = .);
		KEEP(*(SORT_BY_INIT_PRIORITY(.fini_array.*)))
		KEEP(*(.fini_array))
		PROVIDE(__fini_array_end = .);
	}

	/* Uninitialized data section. */
	.bss :
	{
		*(.bss*)
	}

	/* Read-only data section. */
	.rodata : ALIGN(PAGE_SIZE)
	{
		*(.rodata*)
	}

	/* Dynamic symbol and string tables. */
	.dynsym : ALIGN(8)
	{
		PROVIDE(__dynsym_start = .);
		*(.dynsym)
		PROVIDE(__dynsym_end = .);
	}

	.dynstr : ALIGN(8)
	{
		PROVIDE(__dynstr_start = .);
		*(.dynstr)
		PROVIDE(__dynstr_end = .);
	}

	.hash : ALIGN(8)
	{
		*(.hash)
		*(.gnu.hash)
	}

	/* Exception handling metadata. */
	.eh_frame_hdr : ALIGN(8)
	{
		PROVIDE(__eh_frame_hdr_start = .);
		KEEP(*(.eh_frame_hdr))
		PROVIDE(__eh_frame_hdr_end = .);
	}

	.eh_frame : ALIGN(8)
	{
		PROVIDE(__eh_frame_start = .);
		KEEP(*(.eh_frame))
		PROVIDE(__eh_frame_end = .);
	}

	/* Thread-local storage image. */
	.tls : ALIGN(PAGE_SIZE)
	{
		__TLS_START = .;
		*(.tdata)
		*(.tbss)
		__TLS_END = .;
	}

	/* Ensure the NOBITS TLS tail is covered by a loadable segment. */
	.tls_init ADDR(.tls) : AT(ADDR(.tls))
	{
		FILL(0x0000000000000000);
		. = __TLS_END;
	}

	/* Discarded. */
	/DISCARD/ :
	{
		*(.comment)
		*(.note)
	}
}
