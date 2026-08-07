// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # ELF64 Constants and Structures
//!
//! This module provides shared ELF64 structures and validation logic used across
//! kernel, uservm, and guest library ELF loaders.
//!

//==================================================================================================
// Imports
//==================================================================================================

use super::elf32::{
    EI_CLASS,
    EI_DATA,
    EI_NIDENT,
    ELFCLASS64,
    ELFDATA2LSB,
    ELFMAG0,
    ELFMAG1,
    ELFMAG2,
    ELFMAG3,
    EV_CURRENT,
    PT_LOAD,
};

//==================================================================================================
// Constants
//==================================================================================================

/// AMD x86-64 machine type.
pub const EM_X86_64: u16 = 62;

/// AArch64 machine type.
pub const EM_AARCH64: u16 = 183;

//==================================================================================================
// ELF64 File Header
//==================================================================================================

/// ELF64 file header.
#[repr(C)]
pub struct Elf64Fhdr {
    /// ELF magic numbers and other info.
    pub e_ident: [u8; EI_NIDENT],
    /// Object file type.
    pub e_type: u16,
    /// Required machine architecture type.
    pub e_machine: u16,
    /// Object file version.
    pub e_version: u32,
    /// Virtual address of process's entry point.
    pub e_entry: u64,
    /// Program header table file offset.
    pub e_phoff: u64,
    /// Section header table file offset.
    pub e_shoff: u64,
    /// Processor-specific flags.
    pub e_flags: u32,
    /// ELF header's size in bytes.
    pub e_ehsize: u16,
    /// Program header table entry size.
    pub e_phentsize: u16,
    /// Entries in the program header table.
    pub e_phnum: u16,
    /// Section header table size.
    pub e_shentsize: u16,
    /// Entries in the section header table.
    pub e_shnum: u16,
    /// Index for the section name string table.
    pub e_shstrndx: u16,
}

/// Type alias for compatibility with ELF naming conventions.
pub type Elf64Ehdr = Elf64Fhdr;

impl Elf64Fhdr {
    ///
    /// # Description
    ///
    /// Interprets the memory at the given address as an [`Elf64Fhdr`].
    ///
    /// # Parameters
    ///
    /// - `addr`: Starting address of the ELF64 file header.
    ///
    /// # Returns
    ///
    /// A reference to the [`Elf64Fhdr`] located at `addr`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `addr` points to a valid, properly aligned `Elf64Fhdr` that
    /// outlives the returned reference.
    ///
    pub unsafe fn from_address<'a>(addr: usize) -> &'a Self {
        debug_assert!(addr.is_multiple_of(core::mem::align_of::<Self>()), "unaligned Elf64Fhdr");
        unsafe { &*(addr as *const Self) }
    }

    ///
    /// # Description
    ///
    /// Validates the ELF magic number in the header.
    ///
    /// # Returns
    ///
    /// `true` if the magic bytes match the ELF specification, `false` otherwise.
    ///
    pub fn is_valid(&self) -> bool {
        self.e_ident[0] == ELFMAG0
            && self.e_ident[1] == ELFMAG1
            && self.e_ident[2] == ELFMAG2
            && self.e_ident[3] == ELFMAG3
    }

    ///
    /// # Description
    ///
    /// Validates the ELF64 header fields common to all loaders: magic bytes, 64-bit class,
    /// little-endian encoding, current version, and program header entry size.
    ///
    /// Object file type (`e_type`) and machine architecture (`e_machine`) are intentionally
    /// excluded because different consumers expect different values.
    ///
    /// # Returns
    ///
    /// `Ok(())` if all checks pass, or `Err` with a static reason string describing the first
    /// failing check.
    ///
    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.is_valid() {
            return Err("invalid ELF magic");
        }
        if self.e_ident[EI_CLASS] != ELFCLASS64 {
            return Err("invalid ELF class");
        }
        if self.e_ident[EI_DATA] != ELFDATA2LSB {
            return Err("invalid data encoding");
        }
        if self.e_version != EV_CURRENT {
            return Err("invalid ELF version");
        }
        if self.e_phentsize as usize != core::mem::size_of::<Elf64Phdr>() {
            return Err("invalid program header entry size");
        }
        Ok(())
    }
}

//==================================================================================================
// ELF64 Program Header
//==================================================================================================

/// ELF64 program header.
#[repr(C)]
pub struct Elf64Phdr {
    /// Segment type.
    pub p_type: u32,
    /// Segment flags.
    pub p_flags: u32,
    /// Offset of the first byte.
    pub p_offset: u64,
    /// Virtual address of the first byte.
    pub p_vaddr: u64,
    /// Physical address of the first byte.
    pub p_paddr: u64,
    /// Bytes in the file image.
    pub p_filesz: u64,
    /// Bytes in the memory image.
    pub p_memsz: u64,
    /// Alignment value.
    pub p_align: u64,
}

impl Elf64Phdr {
    ///
    /// # Description
    ///
    /// Returns `true` if this program header describes a loadable segment (`PT_LOAD`).
    ///
    pub fn is_loadable(&self) -> bool {
        self.p_type == PT_LOAD
    }

    ///
    /// # Description
    ///
    /// Validates segment size invariants.
    ///
    /// The caller is responsible for checking the segment type (e.g., via [`Self::is_loadable()`])
    /// before calling this method.
    ///
    /// # Returns
    ///
    /// `Ok(())` if `p_filesz <= p_memsz`, or `Err` with a static reason string.
    ///
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.p_filesz > self.p_memsz {
            return Err("segment file size exceeds memory size");
        }
        Ok(())
    }
}

//==================================================================================================
// ELF64 Dynamic Section
//==================================================================================================

/// Marks end of dynamic section.
pub const DT_NULL: i64 = 0;
/// Offset in the string table of a needed shared-library name.
pub const DT_NEEDED: i64 = 1;
/// Size in bytes of the PLT relocation table.
pub const DT_PLTRELSZ: i64 = 2;
/// Address of the dynamic symbol table.
pub const DT_SYMTAB: i64 = 6;
/// Address of an RELA relocation table (relocations with explicit addends).
pub const DT_RELA: i64 = 7;
/// Size in bytes of the RELA relocation table.
pub const DT_RELASZ: i64 = 8;
/// Type of relocation entry referenced by `DT_JMPREL` (`DT_REL` or `DT_RELA`).
pub const DT_PLTREL: i64 = 20;
/// Address of the PLT relocation table.
pub const DT_JMPREL: i64 = 23;

/// ELF64 dynamic section entry.
#[repr(C)]
pub struct Elf64Dyn {
    /// Entry type tag.
    pub d_tag: i64,
    /// Integer / address value.
    pub d_val: u64,
}

//==================================================================================================
// ELF64 Relocation Entry
//==================================================================================================

/// Adjust by program base (`R_X86_64_RELATIVE`).
pub const R_X86_64_RELATIVE: u32 = 8;

/// Adjust by program base (`R_AARCH64_RELATIVE`).
pub const R_AARCH64_RELATIVE: u32 = 1027;

/// ELF64 relocation entry with explicit addend.
#[repr(C)]
pub struct Elf64Rela {
    /// Offset at which to apply the relocation.
    pub r_offset: u64,
    /// Relocation type (low 32 bits) and symbol index (high 32 bits).
    pub r_info: u64,
    /// Constant addend used to compute the relocated value.
    pub r_addend: i64,
}
