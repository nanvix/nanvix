/*
 * Copyright(c) 2011-2024 The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

OUTPUT_FORMAT("elf32-i386")
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

	/*
	 * Dynamic section.
	 *
	 * Holds the DT_* entries (DT_NEEDED dependencies and the addresses/sizes of
	 * the dynamic relocation and symbol tables). The bounding symbols are
	 * provided HERE — mirroring `.dynsym`/`.dynstr` below — so the dlfcn
	 * self-linker (`syscall::dlfcn::dllink_executable`) can find `.dynamic` at
	 * runtime: the ELF header and program headers are not mapped at the image's
	 * load address, so they cannot be parsed in-process. The boundaries collapse
	 * to an empty range for statically linked images (no `.dynamic`), making the
	 * self-linker a no-op there.
	 */
	.dynamic : ALIGN(4)
	{
		PROVIDE(__dynamic_start = .);
		*(.dynamic)
		PROVIDE(__dynamic_end = .);
	}

	/* Global constructors. */
	.ctors : ALIGN(4)
	{
		KEEP(*crtbegin*.o(.ctors))
		KEEP(*(EXCLUDE_FILE(*crtend*.o) .ctors))
		KEEP(*(SORT(.ctors.*)))
		KEEP(*crtend*.o(.ctors))
	}

	/* Global destructors. */
	.dtors : ALIGN(4)
	{
		KEEP(*crtbegin*.o(.dtors))
		KEEP(*(EXCLUDE_FILE(*crtend*.o) .dtors))
		KEEP(*(SORT(.dtors.*)))
		KEEP(*crtend*.o(.dtors))
	}

	/*
	 * Constructor / destructor arrays (the primary ctor/dtor mechanism).
	 *
	 * `__nanvix_libc_start_main` (libposix start.rs) walks `.preinit_array`
	 * then `.init_array` before calling `main`, and `.fini_array` (in reverse)
	 * after `main` returns.  The bound symbols below are provided HERE — not by
	 * GCC crtbegin/crtend — because Nanvix ships no crti/crtn/crtbegin/crtend.
	 * These sections hold relocated function pointers, so they must live in the
	 * writable LOAD segment (with .data/.ctors/.dtors) where the PIE relocator
	 * fixes them up.
	 */
	.preinit_array : ALIGN(4)
	{
		PROVIDE_HIDDEN(__preinit_array_start = .);
		KEEP(*(.preinit_array))
		PROVIDE_HIDDEN(__preinit_array_end = .);
	}

	.init_array : ALIGN(4)
	{
		PROVIDE_HIDDEN(__init_array_start = .);
		KEEP(*(SORT_BY_INIT_PRIORITY(.init_array.*)))
		KEEP(*(.init_array))
		PROVIDE_HIDDEN(__init_array_end = .);
	}

	.fini_array : ALIGN(4)
	{
		PROVIDE_HIDDEN(__fini_array_start = .);
		KEEP(*(SORT_BY_INIT_PRIORITY(.fini_array.*)))
		KEEP(*(.fini_array))
		PROVIDE_HIDDEN(__fini_array_end = .);
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

	/* Dynamic symbol table (populated by --export-dynamic). */
	.dynsym : ALIGN(4)
	{
		PROVIDE(__dynsym_start = .);
		*(.dynsym)
		PROVIDE(__dynsym_end = .);
	}

	/* Dynamic string table (symbol names for .dynsym). */
	.dynstr : ALIGN(4)
	{
		PROVIDE(__dynstr_start = .);
		*(.dynstr)
		PROVIDE(__dynstr_end = .);
	}

	/* Symbol hash table. */
	.hash : ALIGN(4)
	{
		*(.hash)
		*(.gnu.hash)
	}

	/* Exception handling frame header */
	.eh_frame_hdr : ALIGN(4)
	{
		PROVIDE(__eh_frame_hdr_start = .);
		KEEP(*(.eh_frame_hdr))
		PROVIDE(__eh_frame_hdr_end = .);
	}

	/* Exception handling frames */
	.eh_frame : ALIGN(4)
	{
		PROVIDE(__eh_frame_start = .);
		KEEP(*(.eh_frame))
		PROVIDE(__eh_frame_end = .);
	}

	/*
	 * Thread-local storage.
	 *
	 * Must appear after all PROGBITS sections (.eh_frame, etc.) so that the NOBITS .tbss
	 * sub-section does not cause virtual-address (VMA) overlap with subsequent PROGBITS sections.
	 */
	.tls : ALIGN(PAGE_SIZE)
	{
		__TLS_START = .;
		*(.tdata)
		*(.tbss)
		__TLS_END = .;
	}

	/*
	 * PROGBITS companion for .tls.
	 *
	 * Because .tbss is NOBITS it does not produce a LOAD segment.  The TDA allocator (tda.rs)
	 * copies __TLS_START..__TLS_END into each new thread, so the kernel must have allocated and
	 * zero-filled the pages backing that range.  This zero-filled PROGBITS section creates a LOAD
	 * segment that covers the .tbss region, causing the ELF loader to map and clear those pages.
	 */
	.tls_init ADDR(.tls) : AT(ADDR(.tls))
	{
		FILL(0x00000000);
		. = __TLS_END;
	}

	/* Discarded. */
	/DISCARD/ :
	{
		*(.comment)
		*(.note)
	}
}
