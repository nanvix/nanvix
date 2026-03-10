// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # ELF32 Constants and Structures
//!
//! This module provides shared ELF32 constants, structures, and validation logic used across
//! kernel, uservm, and guest library ELF loaders.
//!

//==================================================================================================
// ELF Identification
//==================================================================================================

/// Number of identification bytes in ELF header.
pub const EI_NIDENT: usize = 16;

/// ELF magic number byte 0.
pub const ELFMAG0: u8 = 0x7f;
/// ELF magic number byte 1.
pub const ELFMAG1: u8 = b'E';
/// ELF magic number byte 2.
pub const ELFMAG2: u8 = b'L';
/// ELF magic number byte 3.
pub const ELFMAG3: u8 = b'F';

//==================================================================================================
// File Classes
//==================================================================================================

/// Invalid class.
pub const ELFCLASSNONE: u8 = 0;
/// 32-bit object.
pub const ELFCLASS32: u8 = 1;
/// 64-bit object.
pub const ELFCLASS64: u8 = 2;

//==================================================================================================
// Data Encoding Types
//==================================================================================================

/// Invalid data encoding.
pub const ELFDATANONE: u8 = 0;
/// Least significant byte in the lowest address.
pub const ELFDATA2LSB: u8 = 1;
/// Most significant byte in the lowest address.
pub const ELFDATA2MSB: u8 = 2;

//==================================================================================================
// Segment Permissions
//==================================================================================================

/// Segment is executable.
pub const PF_X: u32 = 1 << 0;
/// Segment is writable.
pub const PF_W: u32 = 1 << 1;
/// Segment is readable.
pub const PF_R: u32 = 1 << 2;

//==================================================================================================
// Object File Types
//==================================================================================================

/// No file type.
pub const ET_NONE: u16 = 0;
/// Relocatable file.
pub const ET_REL: u16 = 1;
/// Executable file.
pub const ET_EXEC: u16 = 2;
/// Shared object file.
pub const ET_DYN: u16 = 3;
/// Core file.
pub const ET_CORE: u16 = 4;
/// Processor-specific (low bound).
pub const ET_LOPROC: u16 = 0xff00;
/// Processor-specific (high bound).
pub const ET_HIPROC: u16 = 0xffff;

//==================================================================================================
// Required Machine Architecture Types
//==================================================================================================

/// No machine.
pub const EM_NONE: u16 = 0;
/// AT&T WE 32100.
pub const EM_M32: u16 = 1;
/// SPARC.
pub const EM_SPARC: u16 = 2;
/// Intel 80386.
pub const EM_386: u16 = 3;
/// Motorola 68000.
pub const EM_68K: u16 = 4;
/// Motorola 88000.
pub const EM_88K: u16 = 5;
/// Intel 80860.
pub const EM_860: u16 = 7;
/// MIPS RS3000.
pub const EM_MIPS: u16 = 8;

//==================================================================================================
// Object File Versions
//==================================================================================================

/// Invalid version.
pub const EV_NONE: u32 = 0;
/// Current version.
pub const EV_CURRENT: u32 = 1;

//==================================================================================================
// Segment Types
//==================================================================================================

/// Unused segment.
pub const PT_NULL: u32 = 0;
/// Loadable segment.
pub const PT_LOAD: u32 = 1;
/// Dynamic linking.
pub const PT_DYNAMIC: u32 = 2;
/// Interpreter.
pub const PT_INTERP: u32 = 3;
/// Auxiliary information.
pub const PT_NOTE: u32 = 4;
/// Reserved.
pub const PT_SHLIB: u32 = 5;
/// Program header table.
pub const PT_PHDR: u32 = 6;
/// Low limit for processor-specific.
pub const PT_LOPROC: u32 = 0x70000000;
/// High limit for processor-specific.
pub const PT_HIPROC: u32 = 0x7fffffff;

//==================================================================================================
// Dynamic Section Tags
//==================================================================================================

/// Marks end of dynamic section.
pub const DT_NULL: i32 = 0;
/// Address of relocation table.
pub const DT_REL: i32 = 17;
/// Size of relocation table in bytes.
pub const DT_RELSZ: i32 = 18;
/// Address of PLT relocation table.
pub const DT_JMPREL: i32 = 23;
/// Size of PLT relocation table in bytes.
pub const DT_PLTRELSZ: i32 = 2;

//==================================================================================================
// Relocation Types
//==================================================================================================

/// Adjust by program base (R_386_RELATIVE).
pub const R_386_RELATIVE: u32 = 8;

//==================================================================================================
// ELF32 File Header
//==================================================================================================

/// ELF32 file header.
#[repr(C)]
pub struct Elf32Fhdr {
    /// ELF magic numbers and other info.
    pub e_ident: [u8; EI_NIDENT],
    /// Object file type.
    pub e_type: u16,
    /// Required machine architecture type.
    pub e_machine: u16,
    /// Object file version.
    pub e_version: u32,
    /// Virtual address of process's entry point.
    pub e_entry: u32,
    /// Program header table file offset.
    pub e_phoff: u32,
    /// Section header table file offset.
    pub e_shoff: u32,
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
pub type Elf32Ehdr = Elf32Fhdr;

impl Elf32Fhdr {
    ///
    /// # Description
    ///
    /// Interprets the memory at the given address as an [`Elf32Fhdr`].
    ///
    /// # Parameters
    ///
    /// - `addr`: Starting address of the ELF32 file header.
    ///
    /// # Returns
    ///
    /// A reference to the [`Elf32Fhdr`] located at `addr`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `addr` points to a valid, properly aligned `Elf32Fhdr` that
    /// outlives the returned reference.
    ///
    pub unsafe fn from_address<'a>(addr: usize) -> &'a Self {
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
    /// Validates the ELF32 header fields common to all loaders: magic bytes, 32-bit class,
    /// little-endian encoding, current version, and program header entry size.
    ///
    /// Object file type (`e_type`) and machine architecture (`e_machine`) are intentionally
    /// excluded because different consumers expect different values (e.g., `ET_EXEC` vs `ET_DYN`).
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
        if self.e_ident[4] != ELFCLASS32 {
            return Err("invalid ELF class");
        }
        if self.e_ident[5] != ELFDATA2LSB {
            return Err("invalid data encoding");
        }
        if self.e_version != EV_CURRENT {
            return Err("invalid ELF version");
        }
        if self.e_phentsize as usize != core::mem::size_of::<Elf32Phdr>() {
            return Err("invalid program header entry size");
        }
        Ok(())
    }
}

//==================================================================================================
// ELF32 Program Header
//==================================================================================================

/// ELF32 program header.
#[repr(C)]
pub struct Elf32Phdr {
    /// Segment type.
    pub p_type: u32,
    /// Offset of the first byte.
    pub p_offset: u32,
    /// Virtual address of the first byte.
    pub p_vaddr: u32,
    /// Physical address of the first byte.
    pub p_paddr: u32,
    /// Bytes in the file image.
    pub p_filesz: u32,
    /// Bytes in the memory image.
    pub p_memsz: u32,
    /// Segment flags.
    pub p_flags: u32,
    /// Alignment value.
    pub p_align: u32,
}

impl Elf32Phdr {
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
    /// Validates a loadable segment's size invariants.
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
// ELF32 Dynamic Section Entry
//==================================================================================================

/// ELF32 dynamic section entry.
#[repr(C)]
pub struct Elf32Dyn {
    /// Entry type tag.
    pub d_tag: i32,
    /// Integer value.
    pub d_val: u32,
}

//==================================================================================================
// ELF32 Relocation Entry
//==================================================================================================

/// ELF32 relocation entry (without addend).
#[repr(C)]
pub struct Elf32Rel {
    /// Offset at which to apply the relocation.
    pub r_offset: u32,
    /// Relocation type and symbol index.
    pub r_info: u32,
}
