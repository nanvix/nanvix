// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # ELF File Parser
//!
//! This module provides a simple parser for ELF files.
//!

//==================================================================================================
// Lint Exceptions
//==================================================================================================

// Not all functions are used.
#![allow(dead_code)]

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::std::mem;
use ::syslog::{
    debug,
    error,
    trace,
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

///
/// # Description
///
/// Loads an ELF file into memory.
///
/// # Parameters
///
/// - `destination`: Destination address in memory.
/// - `source`: Source address in memory.
/// - `max_offset`: Maximum offset in memory.
///
/// # Returns
///
/// Upon successful completion, this function returns a tuple containing the entry point, the first
/// address, and the size of the program that was loaded into memory. Otherwise, it returns an error.
///
/// # Safety
///
/// This function is unsafe because it manipulates raw pointers and is up to the caller to ensure
/// that the following conditions are met:
///
/// - The `destination` address is valid.
/// - The `source` address is valid.
/// - The `max_offset` is valid.
///
#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe fn load(
    destination: *mut std::ffi::c_void,
    source: *const u8,
    max_offset: usize,
) -> Result<(usize, usize, usize)> {
    trace!(
        "load(): destination={:?} source={:?} max_offset={max_offset:#010x}",
        destination, source
    );

    let mut first_address: usize = usize::MAX;
    let mut last_address: usize = 0;

    // Get entry point.
    let ehdr: *const Elf32Fhdr = source.cast::<Elf32Fhdr>();

    let entry: usize = (*ehdr).e_entry as usize;
    trace!("entry point: {entry:#010x}");

    // Check if ELF magic number is valid.
    if (*ehdr).e_ident[0] != ELFMAG0
        || (*ehdr).e_ident[1] != ELFMAG1 as u8
        || (*ehdr).e_ident[2] != ELFMAG2 as u8
        || (*ehdr).e_ident[3] != ELFMAG3 as u8
    {
        let reason: String = "header is null or invalid magic".to_string();
        error!("load(): {reason} (e_ident={:?})", (*ehdr).e_ident);
        return Err(anyhow::anyhow!(reason));
    }

    // Check ELF class.
    if (*ehdr).e_ident[4] != ELFCLASS32 {
        let reason: String = "invalid elf class".to_string();
        error!("load(): {reason} (e_ident={:?})", (*ehdr).e_ident);
        return Err(anyhow::anyhow!(reason));
    }

    // Check data encoding.
    if (*ehdr).e_ident[5] != ELFDATA2LSB {
        let reason: String = "invalid data encoding".to_string();
        error!("load(): {reason} (e_ident={:?})", (*ehdr).e_ident);
        return Err(anyhow::anyhow!(reason));
    }

    // Check version.
    if (*ehdr).e_version != EV_CURRENT {
        let reason: String = "invalid version".to_string();
        error!("load(): {reason} (e_version={})", (*ehdr).e_version);
        return Err(anyhow::anyhow!(reason));
    }

    // Check ELF type.
    if (*ehdr).e_type != ET_EXEC {
        let reason: String = "invalid elf type".to_string();
        error!("load(): {reason} (e_type={})", (*ehdr).e_type);
        return Err(anyhow::anyhow!(reason));
    }

    // Check ELF machine architecture.
    if (*ehdr).e_machine != EM_386 {
        let reason: String = "invalid machine architecture".to_string();
        error!("load(): {reason} (e_machine={})", (*ehdr).e_machine);
        return Err(anyhow::anyhow!(reason));
    }

    // Get program header table.
    let phdr: *const Elf32Phdr = (source as usize + (*ehdr).e_phoff as usize) as *const Elf32Phdr;

    // Check if program header has an invalid size.
    if (*ehdr).e_phentsize as usize != mem::size_of::<Elf32Phdr>() {
        let reason: String = "invalid program header entry size".to_string();
        error!("load(): {reason} (e_phentsize={})", (*ehdr).e_phentsize);
        return Err(anyhow::anyhow!(reason));
    }

    // Load program segments.
    let mut loaded_segment: bool = false;
    for i in 0..(*ehdr).e_phnum {
        let phdr = &*phdr.add(i as usize);

        // Loadable segment.
        if phdr.p_type == PT_LOAD {
            let offset: usize = phdr.p_offset as usize;
            let vaddr: usize = phdr.p_vaddr as usize;
            let filesz: usize = phdr.p_filesz as usize;
            let memsz: usize = phdr.p_memsz as usize;

            // Check if file size exceeds memory size.
            if filesz > memsz {
                let reason: String = "segment file size exceeds memory size".to_string();
                error!("load(): {reason} (filesz={filesz:#010x}, memsz={memsz:#010x})",);
                return Err(anyhow::anyhow!(reason));
            }

            // Check if segment fits in memory.
            if vaddr + memsz > max_offset {
                let reason: String = "segment does not fit in memory".to_string();
                error!(
                    "load(): {reason} (vaddr={vaddr:#010x}, memsz={memsz:#010x}, \
                     max_offset={max_offset:#010x})",
                );
                return Err(anyhow::anyhow!(reason));
            }

            debug!(
                "loading(): loading segment: offset={offset:#010x} vaddr={vaddr:#010x} \
                 filesz={filesz:#010x} memsz={memsz:#010x}",
            );

            // Copy segment to memory.
            let src: *const u8 = ehdr.cast::<u8>();
            let src: *const u8 = src.add(offset);
            let dst: *mut u8 = destination.cast::<u8>();
            let dst: *mut u8 = dst.add(vaddr);
            std::ptr::copy_nonoverlapping(src, dst, filesz);

            // Zero out the BSS section.
            if memsz > filesz {
                let bss_size: usize = memsz - filesz;
                let bss_dst: *mut u8 = dst.add(filesz);
                std::ptr::write_bytes(bss_dst, 0, bss_size);
            }

            // Update first address.
            if !loaded_segment || vaddr < first_address {
                first_address = vaddr;
            }

            // Update last address.
            if vaddr + memsz > last_address {
                last_address = vaddr + memsz;
            }

            loaded_segment = true;
        }
    }

    // Check if no loadable segments were found.
    if !loaded_segment {
        let reason: String = "no loadable segments found".to_string();
        error!("load(): {reason} (e_phnum={})", (*ehdr).e_phnum);
        return Err(anyhow::anyhow!(reason));
    }

    let size: usize = last_address - first_address;

    Ok((entry, first_address, size))
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::std::ffi::c_void;

    const ENTRY_POINT: u32 = 0x0040_2000;
    const VADDR: usize = 0x0010_0000;

    fn make_ident() -> [u8; EI_NIDENT] {
        let mut ident: [u8; EI_NIDENT] = [0; EI_NIDENT];
        ident[0] = ELFMAG0;
        ident[1] = ELFMAG1 as u8;
        ident[2] = ELFMAG2 as u8;
        ident[3] = ELFMAG3 as u8;
        ident[4] = ELFCLASS32;
        ident[5] = ELFDATA2LSB;
        ident[6] = EV_CURRENT as u8;
        ident
    }

    unsafe fn write_struct<T>(buffer: &mut [u8], value: &T) {
        let size: usize = mem::size_of::<T>();
        let src: *const u8 = (value as *const T).cast::<u8>();
        unsafe {
            std::ptr::copy_nonoverlapping(src, buffer.as_mut_ptr(), size);
        }
    }

    #[test]
    fn load_copies_segment_and_zeroes_bss() -> Result<()> {
        let header_size: usize = mem::size_of::<Elf32Fhdr>();
        let phdr_size: usize = mem::size_of::<Elf32Phdr>();
        let segment_offset: usize = header_size + phdr_size;
        let filesz: usize = 4;
        let memsz: usize = 16;
        let segment_data: [u8; 4] = [0xaa, 0xbb, 0xcc, 0xdd];

        let ident: [u8; EI_NIDENT] = make_ident();
        let header: Elf32Fhdr = Elf32Fhdr {
            e_ident: ident,
            e_type: ET_EXEC,
            e_machine: EM_386,
            e_version: EV_CURRENT,
            e_entry: ENTRY_POINT,
            e_phoff: header_size as u32,
            e_shoff: 0,
            e_flags: 0,
            e_ehsize: header_size as u16,
            e_phentsize: phdr_size as u16,
            e_phnum: 1,
            e_shentsize: 0,
            e_shnum: 0,
            e_shstrndx: 0,
        };

        let phdr: Elf32Phdr = Elf32Phdr {
            p_type: PT_LOAD,
            p_offset: segment_offset as u32,
            p_vaddr: VADDR as u32,
            p_paddr: 0,
            p_filesz: filesz as u32,
            p_memsz: memsz as u32,
            p_flags: PF_R | PF_W,
            p_align: 1,
        };

        let mut image: Vec<u8> = vec![0; segment_offset + filesz];
        unsafe {
            write_struct(&mut image[..header_size], &header);
            write_struct(&mut image[header_size..segment_offset], &phdr);
        }
        image[segment_offset..segment_offset + filesz].copy_from_slice(&segment_data);

        let mut memory: Vec<u8> = vec![0xaa; VADDR + memsz + 0x1000];
        let destination: *mut c_void = memory.as_mut_ptr().cast::<c_void>();

        let (entry, first_address, size): (usize, usize, usize) =
            unsafe { load(destination, image.as_ptr(), memory.len())? };

        assert_eq!(entry, ENTRY_POINT as usize);
        assert_eq!(first_address, VADDR);
        assert_eq!(size, memsz);
        assert_eq!(&memory[VADDR..VADDR + filesz], &segment_data);
        assert!(
            memory[VADDR + filesz..VADDR + memsz]
                .iter()
                .all(|byte: &u8| *byte == 0)
        );

        Ok(())
    }

    #[test]
    fn load_rejects_invalid_program_header_size() {
        let header_size: usize = mem::size_of::<Elf32Fhdr>();
        let mut ident: [u8; EI_NIDENT] = make_ident();
        ident[4] = ELFCLASS32;

        let header: Elf32Fhdr = Elf32Fhdr {
            e_ident: ident,
            e_type: ET_EXEC,
            e_machine: EM_386,
            e_version: EV_CURRENT,
            e_entry: ENTRY_POINT,
            e_phoff: header_size as u32,
            e_shoff: 0,
            e_flags: 0,
            e_ehsize: header_size as u16,
            e_phentsize: (mem::size_of::<Elf32Phdr>() as u16) - 1,
            e_phnum: 1,
            e_shentsize: 0,
            e_shnum: 0,
            e_shstrndx: 0,
        };

        let mut image: Vec<u8> = vec![0; header_size];
        unsafe {
            write_struct(&mut image[..], &header);
        }

        let mut memory: Vec<u8> = vec![0; VADDR + 0x100];
        let destination: *mut c_void = memory.as_mut_ptr().cast::<c_void>();

        let result: Result<(usize, usize, usize)> =
            unsafe { load(destination, image.as_ptr(), memory.len()) };

        assert!(result.is_err());
    }

    #[test]
    fn load_rejects_segment_with_filesz_exceeding_memsz() {
        let header_size: usize = mem::size_of::<Elf32Fhdr>();
        let phdr_size: usize = mem::size_of::<Elf32Phdr>();
        let segment_offset: usize = header_size + phdr_size;
        let filesz: usize = 16;
        let memsz: usize = 8;
        let segment_data: [u8; 16] = [0x11; 16];

        let header: Elf32Fhdr = Elf32Fhdr {
            e_ident: make_ident(),
            e_type: ET_EXEC,
            e_machine: EM_386,
            e_version: EV_CURRENT,
            e_entry: ENTRY_POINT,
            e_phoff: header_size as u32,
            e_shoff: 0,
            e_flags: 0,
            e_ehsize: header_size as u16,
            e_phentsize: phdr_size as u16,
            e_phnum: 1,
            e_shentsize: 0,
            e_shnum: 0,
            e_shstrndx: 0,
        };

        let phdr: Elf32Phdr = Elf32Phdr {
            p_type: PT_LOAD,
            p_offset: segment_offset as u32,
            p_vaddr: VADDR as u32,
            p_paddr: 0,
            p_filesz: filesz as u32,
            p_memsz: memsz as u32,
            p_flags: PF_R | PF_W,
            p_align: 1,
        };

        let mut image: Vec<u8> = vec![0; segment_offset + filesz];
        unsafe {
            write_struct(&mut image[..header_size], &header);
            write_struct(&mut image[header_size..segment_offset], &phdr);
        }
        image[segment_offset..segment_offset + filesz].copy_from_slice(&segment_data);

        let mut memory: Vec<u8> = vec![0; VADDR + filesz + 0x100];
        let destination: *mut c_void = memory.as_mut_ptr().cast::<c_void>();

        let result: Result<(usize, usize, usize)> =
            unsafe { load(destination, image.as_ptr(), memory.len()) };

        assert!(result.is_err());
    }
}
