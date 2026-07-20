/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

OUTPUT_FORMAT("elf64-x86-64")
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
	.dynamic : ALIGN(8)
	{
		PROVIDE(__dynamic_start = .);
		*(.dynamic)
		PROVIDE(__dynamic_end = .);
	}

	/*
	 * Constructor / destructor arrays (the primary ctor/dtor mechanism).
	 *
	 * `__nanvix_libc_start_main` (nanvix_libc start.rs) walks `.preinit_array`
	 * then `.init_array` before calling `main`, and `.fini_array` (in reverse)
	 * after `main` returns.  The bound symbols below are provided HERE — not by
	 * GCC crtbegin/crtend — because Nanvix ships no crti/crtn/crtbegin/crtend.
	 * These sections hold relocated function pointers, so they must live in the
	 * writable LOAD segment (with .data) where the PIE relocator fixes them up.
	 *
	 * The boundary symbols use `PROVIDE` (default visibility) rather than the
	 * customary `PROVIDE_HIDDEN`, mirroring the `__dynamic_*` / `__dynsym_*` /
	 * `__dynstr_*` bounds above, so they are emitted into `.dynsym` under
	 * `--export-dynamic`. A dynamically linked `libc.so` references these section
	 * bounds as undefined symbols (its own `__nanvix_libc_start_main` walks the
	 * arrays), and the startup DT_NEEDED loader
	 * (`syscall::dlfcn::dllink_executable`) resolves them from the executable's
	 * exported global scope when it auto-loads `libc.so`.
	 */
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

	/* Dynamic symbol table (populated by --export-dynamic). */
	.dynsym : ALIGN(8)
	{
		PROVIDE(__dynsym_start = .);
		*(.dynsym)
		PROVIDE(__dynsym_end = .);
	}

	/* Dynamic string table (symbol names for .dynsym). */
	.dynstr : ALIGN(8)
	{
		PROVIDE(__dynstr_start = .);
		*(.dynstr)
		PROVIDE(__dynstr_end = .);
	}

	/* Symbol hash table. */
	.hash : ALIGN(8)
	{
		*(.hash)
		*(.gnu.hash)
	}

	/* Exception handling frame header */
	.eh_frame_hdr : ALIGN(8)
	{
		PROVIDE(__eh_frame_hdr_start = .);
		KEEP(*(.eh_frame_hdr))
		PROVIDE(__eh_frame_hdr_end = .);
	}

	/* Exception handling frames */
	.eh_frame : ALIGN(8)
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
