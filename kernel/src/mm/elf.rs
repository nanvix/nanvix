// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Exceptions
//==================================================================================================

// Not all functions are used.
#![allow(dead_code)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::mem,
    hal::{
        arch::x86::mem::mmu,
        mem::{
            AccessPermission,
            Address,
            PageAligned,
            PhysicalAddress,
            VirtualAddress,
        },
    },
    klib::{
        self,
        Alignment,
    },
    mm::{
        self,
        VirtMemoryManager,
        Vmem,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Constants
//==================================================================================================

// Number of indented elements in ELF header.
const EI_NIDENT: usize = 16;

// ELF magic numbers.
const ELFMAG0: u8 = 0x7f; // ELF magic number 0.
const ELFMAG1: char = 'E'; // ELF magic number 1.
const ELFMAG2: char = 'L'; // ELF magic number 2.
const ELFMAG3: char = 'F'; // ELF magic number 3.

// File classes.
const ELFCLASSNONE: u8 = 0; // Invalid class.
const ELFCLASS32: u8 = 1; // 32-bit object.
const ELFCLASS64: u8 = 2; // 64-bit object.

// Data encoding types.
const ELFDATANONE: u8 = 0; // Invalid data encoding.
const ELFDATA2LSB: u8 = 1; // Least significant byte in the lowest address.
const ELFDATA2MSB: u8 = 2; // Most significant byte in the lowest address.

// Segment permissions.
const PF_X: u32 = 1 << 0; // Segment is executable.
const PF_W: u32 = 1 << 1; // Segment is writable.
const PF_R: u32 = 1 << 2; // Segment is readable.

// Object file types.
const ET_NONE: u16 = 0; // No file type.
const ET_REL: u16 = 1; // Relocatable file.
const ET_EXEC: u16 = 2; // Executable file.
const ET_DYN: u16 = 3; // Shared object file.
const ET_CORE: u16 = 4; // Core file.
const ET_LOPROC: u16 = 0xff00; // Processor-specific.
const ET_HIPROC: u16 = 0xffff; // Processor-specific.

// Required machine architecture types.
const EM_NONE: u16 = 0; // No machine.
const EM_M32: u16 = 1; // AT&T WE 32100.
const EM_SPARC: u16 = 2; // SPARC.
const EM_386: u16 = 3; // Intel 80386.
const EM_68K: u16 = 4; // Motorola 68000.
const EM_88K: u16 = 5; // Motorola 88000.
const EM_860: u16 = 7; // Intel 80860.
const EM_MIPS: u16 = 8; // MIPS RS3000.

// Object file versions.
const EV_NONE: u32 = 0; // Invalid version.
const EV_CURRENT: u32 = 1; // Current version.

// Segment types.
const PT_NULL: u32 = 0; // Unused segment.
const PT_LOAD: u32 = 1; // Loadable segment.
const PT_DYNAMIC: u32 = 2; // Dynamic linking.
const PT_INTERP: u32 = 3; // Interpreter.
const PT_NOTE: u32 = 4; // Auxiliary information.
const PT_SHLIB: u32 = 5; // Reserved.
const PT_PHDR: u32 = 6; // Program header table.
const PT_LOPROC: u32 = 0x70000000; // Low limit for processor-specific.
const PT_HIPROC: u32 = 0x7fffffff; // High limit for processor-specific.

// ELF 32 file header.
#[repr(C)]
pub struct Elf32Fhdr {
    e_ident: [u8; EI_NIDENT], // ELF magic numbers and other info.
    e_type: u16,              // Object file type.
    e_machine: u16,           // Required machine architecture type.
    e_version: u32,           // Object file version.
    e_entry: u32,             // Virtual address of process's entry point.
    e_phoff: u32,             // Program header table file offset.
    e_shoff: u32,             // Section header table file offset.
    e_flags: u32,             // Processor-specific flags.
    e_ehsize: u16,            // ELF header’s size in bytes.
    e_phentsize: u16,         // Program header table entry size.
    e_phnum: u16,             // Entries in the program header table.
    e_shentsize: u16,         // Section header table size.
    e_shnum: u16,             // Entries in the section header table.
    e_shstrndx: u16,          // Index for the section name string table.
}

impl Elf32Fhdr {
    pub fn from_address(addr: usize) -> &'static Self {
        unsafe { &*(addr as *const Self) }
    }
}

// ELF 32 program header.
#[repr(C)]
struct Elf32Phdr {
    p_type: u32,   // Segment type.
    p_offset: u32, // Offset of the first byte.
    p_vaddr: u32,  // Virtual address of the first byte.
    p_paddr: u32,  // Physical address of the first byte.
    p_filesz: u32, // Bytes in the file image.
    p_memsz: u32,  // Bytes in the memory image.
    p_flags: u32,  // Segment flags.
    p_align: u32,  // Alignment value.
}

// Rust equivalent of the C functions.
impl Elf32Fhdr {
    fn is_valid(&self) -> bool {
        if self.e_ident[0] != ELFMAG0
            || self.e_ident[1] != ELFMAG1 as u8
            || self.e_ident[2] != ELFMAG2 as u8
            || self.e_ident[3] != ELFMAG3 as u8
        {
            error!("header is NULL or invalid magic");
            return false;
        }
        true
    }
}

fn do_elf32_load(
    mm: &mut VirtMemoryManager,
    vmem: &mut Vmem,
    elf: &Elf32Fhdr,
    dry_run: bool,
) -> Result<VirtualAddress, Error> {
    trace!("do_el32_load(): dry_run={}", dry_run);

    if !elf.is_valid() {
        return Err(Error::new(ErrorCode::BadFile, "invalid elf file"));
    }

    let phdr_base = unsafe {
        (elf as *const Elf32Fhdr as *const u8).offset(elf.e_phoff as isize) as *const Elf32Phdr
    };
    let phdrs = unsafe { core::slice::from_raw_parts(phdr_base, elf.e_phnum as usize) };

    for phdr in phdrs {
        if phdr.p_type != PT_LOAD {
            continue;
        }

        if phdr.p_filesz > phdr.p_memsz {
            return Err(Error::new(ErrorCode::BadFile, "corrupted elf file"));
        }

        let align: Alignment = phdr
            .p_align
            .try_into()
            .map_err(|_| Error::new(ErrorCode::BadFile, "invalid alignment value in elf file"))?;
        let mut virt_addr: usize = klib::align_down(phdr.p_vaddr as usize, align);

        let access: AccessPermission = if phdr.p_flags == (PF_R | PF_X) {
            AccessPermission::EXEC
        } else if phdr.p_flags == (PF_R | PF_W) {
            AccessPermission::RDWR
        } else {
            AccessPermission::RDONLY
        };

        let phys_addr_base: usize = unsafe {
            (elf as *const Elf32Fhdr as *const u8).offset(phdr.p_offset as isize) as usize
        };

        let phys_addr_end: usize =
            klib::align_down(phys_addr_base + phdr.p_filesz as usize, mmu::PAGE_ALIGNMENT);

        for phys_addr in (phys_addr_base..=phys_addr_end).step_by(mem::PAGE_SIZE) {
            let vaddr: VirtualAddress = VirtualAddress::new(virt_addr);

            if vaddr < mm::USER_BASE {
                let reason: &str = "invalid load address";
                error!("elf32_load: {}", reason);
                return Err(Error::new(ErrorCode::BadFile, "invalid load address"));
            }

            let paddr: PageAligned<PhysicalAddress> = PageAligned::from_raw_value(phys_addr)?;
            let vaddr: PageAligned<VirtualAddress> = PageAligned::from_address(vaddr)?;

            if !dry_run {
                mm.alloc_upage(vmem, vaddr, access)?;

                // TODO: write a detailed comment about this.
                unsafe { vmem.physcopy(vaddr, paddr)? };
            }

            virt_addr += mem::PAGE_SIZE;
        }
    }

    Ok(VirtualAddress::new(elf.e_entry as usize))
}

pub fn elf32_load(
    mm: &mut VirtMemoryManager,
    vmem: &mut Vmem,
    elf: &Elf32Fhdr,
) -> Result<VirtualAddress, Error> {
    if do_elf32_load(mm, vmem, elf, true).is_err() {
        return Ok(VirtualAddress::new(0));
    }

    do_elf32_load(mm, vmem, elf, false)
}
