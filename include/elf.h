/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#ifndef ELF_H_
#define ELF_H_

	#include <stdint.h>

	/* Number of indented elements in ELF header. */
	#define EI_NIDENT 16

	/* ELF magic number. */
	#define ELFMAG0 0x7f /* ELF magic number 0. */
	#define ELFMAG1 'E'  /* ELF magic number 1 .*/
	#define ELFMAG2 'L'  /* ELF magic number 2 .*/
	#define ELFMAG3 'F'  /* ELF magic number 3 .*/

	/* File classes. */
	#define ELFCLASSNONE 0 /* Invalid class. */
	#define ELFCLASS32   1 /* 32-bit object. */
	#define ELFCLASS64   2 /* 64-bit object. */

	/* Data encoding types. */
	#define ELFDATANONE 0 /* Invalid data encoding. */
	#define ELFDATA2LSB 1 /* Least significant byte in the lowest address */
	#define ELFDATA2MSB 2 /* Most significant byte in the lowest address. */

	#define PF_X (1 << 0)	/* Segment is executable */
	#define PF_W (1 << 1)	/* Segment is writable */
	#define PF_R (1 << 2)	/* Segment is readable */

	/* Object file types. */
	#define ET_NONE 0        /* No file type.       */
	#define ET_REL  1        /* Relocatable file.   */
	#define ET_EXEC 2        /* Executable file.    */
	#define ET_DYN  3        /* Shared object file. */
	#define ET_CORE 4        /* Core file.          */
	#define ET_LOPROC 0xff00 /* Processor-specific. */
	#define ET_HIPROC 0xffff /* Processor-specific. */

	 /* Required machine architecture types. */
	#define EM_NONE  0 /* No machine.     */
	#define EM_M32   1 /* AT&T WE 32100.  */
	#define EM_SPARC 2 /* SPARC.          */
	#define EM_386   3 /* Intel 80386.    */
	#define EM_68K   4 /* Motorola 68000. */
	#define EM_88K   5 /* Motorola 88000. */
	#define EM_860   7 /* Intel 80860.    */
	#define EM_MIPS  8 /* MIPS RS3000.    */

	/* Object file versions. */
	#define EV_NONE    0 /* Invalid version. */
	#define EV_CURRENT 1 /* Current version. */

	/* Segment types. */
	#define PT_NULL             0 /* Unused segment.                    */
	#define PT_LOAD             1 /* Loadable segment.                  */
	#define PT_DYNAMIC          2 /* Dynamic linking.                   */
	#define PT_INTERP           3 /* Interpreter.                       */
	#define PT_NOTE             4 /* Auxiliary information.             */
	#define PT_SHLIB            5 /* Reserved.                          */
	#define PT_PHDR             6 /* Program header table.              */
	#define PT_LOPROC  0x70000000 /* Low limit for processor-specific.  */
	#define PT_HIPROC  0x7fffffff /* High limit for processor-specific. */

	/*
	 * ELF 32 file header.
	 */
	struct elf32_fhdr
	{
		uint8_t e_ident[EI_NIDENT]; /* 0 - ELF magic number 0.
									 * 1 - ELF magic number 1.
									 * 2 - ELF magic number 2.
									 * 3 - ELF magic number 3.
									 * 4 - File class or capacity.
									 * 5 - Data encoding.
									 * 6 - File version.
									 * 7 - Start of padding bytes.
									 * 16 - size of e_ident[].
									 */
		uint16_t e_type;      /* Object file type.                         */
		uint16_t e_machine;   /* Required machine architecture type.       */
		uint32_t e_version;   /* Object file version.                      */
		uint32_t e_entry;     /* Virtual address of process's entry point. */
		uint32_t e_phoff;     /* Program header table file offset.         */
		uint32_t e_shoff;     /* Section header table file offset.         */
		uint32_t e_flags;     /* Processor-specific flags.                 */
		uint16_t e_ehsize;    /* ELF header’s size in bytes.               */
		uint16_t e_phentsize; /* Program header table entry size.          */
		uint16_t e_phnum;     /* Entries in the program header table.      */
		uint16_t e_shentsize; /* Section header table size                 */
		uint16_t e_shnum;     /* Entries in the section header table.      */
		uint16_t e_shstrndx;  /* Index for the section name string table.  */
	};

	/*
	 * ELF 32 program header.
	 */
	struct elf32_phdr
	{
		uint32_t p_type;   /* Segment type.                       */
		uint32_t p_offset; /* Offset of the first byte.           */
		uint32_t p_vaddr;  /* Virtual address of the first byte.  */
		uint32_t p_paddr;  /* Physical address of the first byte. */
		uint32_t p_filesz; /* Bytes in the file image.            */
		uint32_t p_memsz;  /* Bytes in the memory image.          */
		uint32_t p_flags;  /* Segment flags.                      */
		uint32_t p_align;  /* Alignment value.                    */
	};

#endif /* ELF_H_ */
