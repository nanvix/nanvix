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

// Not all constants are used.
#![allow(dead_code)]

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::core::ptr;
use ::log::{
    debug,
    error,
    trace,
};
use ::std::mem;

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
            error!("header is null or invalid magic");
            return false;
        }
        true
    }
}

///
/// # Description
///
/// Represents the virtual memory range occupied by an ELF's loadable segments.
///
/// This structure captures the lowest and highest virtual addresses used by the loadable segments
/// of an ELF file. It is useful for pre-validating memory layout requirements before loading the
/// ELF into a guest.
///
pub struct MemoryFootprint {
    start: usize,
    end: usize,
}

impl MemoryFootprint {
    ///
    /// # Description
    ///
    /// Returns the lowest virtual address used by any loadable segment in the ELF file.
    ///
    pub const fn start(&self) -> usize {
        self.start
    }

    ///
    /// # Description
    ///
    /// Returns the highest virtual address used by the loadable segments in the ELF file.
    ///
    pub const fn end(&self) -> usize {
        self.end
    }

    ///
    /// # Description
    ///
    /// Returns the total virtual address span occupied by the loadable segments.
    ///
    pub const fn size(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

///
/// # Description
///
/// Computes the memory footprint of an ELF file.
///
/// This function parses the ELF header and program headers to determine the lowest and highest
/// virtual addresses that will be occupied when the ELF is loaded into memory. This is useful for
/// calculating memory layout requirements before actually loading the ELF.
///
/// # Parameters
///
/// - `source`: ELF file bytes in memory.
///
/// # Returns
///
/// Upon successful completion, this function returns the memory footprint of the loadable
/// segments. Otherwise, it returns an error.
///
pub fn memory_footprint(source: &[u8]) -> Result<MemoryFootprint> {
    trace!("memory_footprint(): source_len={}", source.len());

    let source_len: usize = source.len();
    let fh_size: usize = mem::size_of::<Elf32Fhdr>();

    // Check if buffer is too small to contain ELF header.
    if source_len < fh_size {
        let reason: &str = "buffer too small for ELF header";
        error!("memory_footprint(): {reason} (len={source_len})");
        return Err(anyhow::anyhow!(reason));
    }

    // Safety: the buffer size check above guarantees that `source` contains at least
    // `size_of::<Elf32Fhdr>()` bytes, so the read is within bounds.
    let ehdr: Elf32Fhdr = unsafe { ptr::read_unaligned(source.as_ptr().cast::<Elf32Fhdr>()) };

    // Check if ELF magic number is valid.
    if !ehdr.is_valid() {
        let reason: &str = "header is null or invalid magic";
        return Err(anyhow::anyhow!(reason));
    }

    // Check ELF class.
    if ehdr.e_ident[4] != ELFCLASS32 {
        let reason: &str = "invalid elf class";
        error!("memory_footprint(): {reason} (e_ident={:?})", ehdr.e_ident);
        return Err(anyhow::anyhow!(reason));
    }

    let phoff: usize = ehdr.e_phoff as usize;
    let phentsize: usize = ehdr.e_phentsize as usize;
    let phnum: usize = ehdr.e_phnum as usize;

    // Check if program header has an invalid size.
    if phentsize != mem::size_of::<Elf32Phdr>() {
        let reason: &str = "invalid program header entry size";
        error!("memory_footprint(): {reason} (e_phentsize={})", ehdr.e_phentsize);
        return Err(anyhow::anyhow!(reason));
    }

    // Calculate program header table size.
    let ph_table_size: usize = phentsize.checked_mul(phnum).ok_or_else(|| {
        let reason: &str = "program header table size overflow";
        error!("memory_footprint(): {reason} (e_phnum={})", ehdr.e_phnum);
        anyhow::anyhow!(reason)
    })?;

    // Calculate end of program header table.
    let ph_table_end: usize = phoff.checked_add(ph_table_size).ok_or_else(|| {
        let reason: &str = "program header table offset overflow";
        error!("memory_footprint(): {reason} (e_phoff={})", ehdr.e_phoff);
        anyhow::anyhow!(reason)
    })?;

    // Check if program header table exceeds buffer.
    if ph_table_end > source_len {
        let reason: &str = "program header table exceeds buffer";
        error!(
            "memory_footprint(): {reason} (phoff={}, size={}, len={})",
            phoff, ph_table_size, source_len
        );
        return Err(anyhow::anyhow!(reason));
    }

    // Find the lowest and highest virtual addresses across all loadable segments.
    let mut end_address: usize = 0;
    let mut start_address: usize = usize::MAX;
    let mut found_loadable: bool = false;

    for i in 0..phnum {
        let entry_offset: usize = phoff + (i * phentsize);
        let entry_end: usize = entry_offset + phentsize;

        // Check if program header entry exceeds buffer.
        if entry_end > source_len {
            let reason: &str = "program header entry exceeds buffer";
            error!(
                "memory_footprint(): {reason} (offset={}, phentsize={}, len={})",
                entry_offset, phentsize, source_len
            );
            return Err(anyhow::anyhow!(reason));
        }

        // Safety: the bounds check above ensures `entry_offset + phentsize <= source_len`,
        // so the pointer arithmetic and subsequent read are within the buffer.
        let phdr_ptr: *const Elf32Phdr =
            unsafe { source.as_ptr().add(entry_offset) }.cast::<Elf32Phdr>();
        let phdr: Elf32Phdr = unsafe { ptr::read_unaligned(phdr_ptr) };

        // Loadable segment.
        if phdr.p_type == PT_LOAD {
            let vaddr: usize = phdr.p_vaddr as usize;
            let memsz: usize = phdr.p_memsz as usize;
            let segment_end: usize = vaddr.checked_add(memsz).ok_or_else(|| {
                let reason: &str = "segment end address overflow";
                error!("memory_footprint(): {reason} (vaddr={vaddr:#010x}, memsz={memsz})");
                anyhow::anyhow!(reason)
            })?;

            // Update start addresses.
            if vaddr < start_address {
                start_address = vaddr;
            }
            // Update end address.
            if segment_end > end_address {
                end_address = segment_end;
            }
            found_loadable = true;
        }
    }

    // Check if no loadable segments were found.
    if !found_loadable {
        let reason: &str = "no loadable segments found";
        error!("memory_footprint(): {reason} (e_phnum={})", ehdr.e_phnum);
        return Err(anyhow::anyhow!(reason));
    }

    // Check if segment layout is invalid.
    if end_address < start_address {
        let reason: &str = "invalid segment layout: start after end";
        error!(
            "memory_footprint(): {reason} (start={start_address:#010x}, end={end_address:#010x})"
        );
        return Err(anyhow::anyhow!(reason));
    }

    debug!("memory_footprint(): start={start_address:#010x}, end={end_address:#010x}");

    Ok(MemoryFootprint {
        start: start_address,
        end: end_address,
    })
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
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ::std::ffi::c_void;

    const ENTRY_POINT: u32 = 0x0040_2000;
    const VADDR: usize = 0x0010_0000;

    /// Converts a `usize` to `u32`, panicking if it does not fit.
    fn to_u32(v: usize) -> u32 {
        u32::try_from(v).expect("value fits in u32")
    }

    /// Converts a `usize` to `u16`, panicking if it does not fit.
    fn to_u16(v: usize) -> u16 {
        u16::try_from(v).expect("value fits in u16")
    }

    fn make_ident() -> [u8; EI_NIDENT] {
        let mut ident: [u8; EI_NIDENT] = [0; EI_NIDENT];
        ident[0] = ELFMAG0;
        ident[1] = u8::try_from(ELFMAG1).expect("ELF magic fits in u8");
        ident[2] = u8::try_from(ELFMAG2).expect("ELF magic fits in u8");
        ident[3] = u8::try_from(ELFMAG3).expect("ELF magic fits in u8");
        ident[4] = ELFCLASS32;
        ident[5] = ELFDATA2LSB;
        ident[6] = u8::try_from(EV_CURRENT).expect("EV_CURRENT fits in u8");
        ident
    }

    /// Builds a standard ELF header for tests.
    fn make_header(e_phnum: u16) -> Elf32Fhdr {
        let header_size: usize = mem::size_of::<Elf32Fhdr>();
        let phdr_size: usize = mem::size_of::<Elf32Phdr>();
        Elf32Fhdr {
            e_ident: make_ident(),
            e_type: ET_EXEC,
            e_machine: EM_386,
            e_version: EV_CURRENT,
            e_entry: ENTRY_POINT,
            e_phoff: to_u32(header_size),
            e_shoff: 0,
            e_flags: 0,
            e_ehsize: to_u16(header_size),
            e_phentsize: to_u16(phdr_size),
            e_phnum,
            e_shentsize: 0,
            e_shnum: 0,
            e_shstrndx: 0,
        }
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

        let header: Elf32Fhdr = make_header(1);

        let phdr: Elf32Phdr = Elf32Phdr {
            p_type: PT_LOAD,
            p_offset: to_u32(segment_offset),
            p_vaddr: to_u32(VADDR),
            p_paddr: 0,
            p_filesz: to_u32(filesz),
            p_memsz: to_u32(memsz),
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

        let mut header: Elf32Fhdr = make_header(1);
        header.e_phentsize = to_u16(mem::size_of::<Elf32Phdr>()) - 1;

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

        let header: Elf32Fhdr = make_header(1);

        let phdr: Elf32Phdr = Elf32Phdr {
            p_type: PT_LOAD,
            p_offset: to_u32(segment_offset),
            p_vaddr: to_u32(VADDR),
            p_paddr: 0,
            p_filesz: to_u32(filesz),
            p_memsz: to_u32(memsz),
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

    //==============================================================================================
    // Tests for memory_footprint()
    //==============================================================================================

    /// Tests that memory_footprint correctly computes start/end for a single loadable segment.
    #[test]
    fn memory_footprint_single_segment() -> Result<()> {
        let header_size: usize = mem::size_of::<Elf32Fhdr>();
        let phdr_size: usize = mem::size_of::<Elf32Phdr>();
        let segment_offset: usize = header_size + phdr_size;

        let header: Elf32Fhdr = make_header(1);

        let phdr: Elf32Phdr = Elf32Phdr {
            p_type: PT_LOAD,
            p_offset: to_u32(segment_offset),
            p_vaddr: to_u32(VADDR),
            p_paddr: 0,
            p_filesz: 0x100,
            p_memsz: 0x200,
            p_flags: PF_R | PF_W,
            p_align: 1,
        };

        let mut image: Vec<u8> = vec![0; segment_offset + 0x100];
        unsafe {
            write_struct(&mut image[..header_size], &header);
            write_struct(&mut image[header_size..segment_offset], &phdr);
        }

        let footprint: MemoryFootprint = memory_footprint(&image)?;

        assert_eq!(footprint.start(), VADDR);
        assert_eq!(footprint.end(), VADDR + 0x200);
        assert_eq!(footprint.size(), 0x200);

        Ok(())
    }

    /// Tests that memory_footprint correctly handles multiple loadable segments.
    #[test]
    fn memory_footprint_multiple_segments() -> Result<()> {
        let header_size: usize = mem::size_of::<Elf32Fhdr>();
        let phdr_size: usize = mem::size_of::<Elf32Phdr>();

        let header: Elf32Fhdr = make_header(2);

        // First segment: starts at 0x1000, size 0x100.
        let phdr1: Elf32Phdr = Elf32Phdr {
            p_type: PT_LOAD,
            p_offset: 0,
            p_vaddr: 0x1000,
            p_paddr: 0,
            p_filesz: 0x100,
            p_memsz: 0x100,
            p_flags: PF_R | PF_X,
            p_align: 1,
        };

        // Second segment: starts at 0x2000, size 0x500.
        let phdr2: Elf32Phdr = Elf32Phdr {
            p_type: PT_LOAD,
            p_offset: 0,
            p_vaddr: 0x2000,
            p_paddr: 0,
            p_filesz: 0x200,
            p_memsz: 0x500,
            p_flags: PF_R | PF_W,
            p_align: 1,
        };

        let mut image: Vec<u8> = vec![0; header_size + 2 * phdr_size];
        unsafe {
            write_struct(&mut image[..header_size], &header);
            write_struct(&mut image[header_size..header_size + phdr_size], &phdr1);
            write_struct(&mut image[header_size + phdr_size..], &phdr2);
        }

        let footprint: MemoryFootprint = memory_footprint(&image)?;

        assert_eq!(footprint.start(), 0x1000);
        assert_eq!(footprint.end(), 0x2500);
        assert_eq!(footprint.size(), 0x1500);

        Ok(())
    }

    /// Tests that memory_footprint rejects a buffer too small for ELF header.
    #[test]
    fn memory_footprint_rejects_small_buffer() {
        let small_buffer: [u8; 10] = [0; 10];

        let result: Result<MemoryFootprint> = memory_footprint(&small_buffer);

        assert!(result.is_err());
    }

    /// Tests that memory_footprint rejects invalid ELF magic.
    #[test]
    fn memory_footprint_rejects_invalid_magic() {
        let header_size: usize = mem::size_of::<Elf32Fhdr>();

        let mut header: Elf32Fhdr = make_header(1);
        header.e_ident = [0; EI_NIDENT]; // Invalid magic.

        let mut image: Vec<u8> = vec![0; header_size];
        unsafe {
            write_struct(&mut image[..], &header);
        }

        let result: Result<MemoryFootprint> = memory_footprint(&image);

        assert!(result.is_err());
    }

    /// Tests that memory_footprint rejects invalid ELF class (64-bit).
    #[test]
    fn memory_footprint_rejects_invalid_class() {
        let header_size: usize = mem::size_of::<Elf32Fhdr>();

        let mut header: Elf32Fhdr = make_header(1);
        header.e_ident[4] = ELFCLASS64; // 64-bit class, not supported.

        let mut image: Vec<u8> = vec![0; header_size];
        unsafe {
            write_struct(&mut image[..], &header);
        }

        let result: Result<MemoryFootprint> = memory_footprint(&image);

        assert!(result.is_err());
    }

    /// Tests that memory_footprint rejects invalid program header entry size.
    #[test]
    fn memory_footprint_rejects_invalid_phentsize() {
        let header_size: usize = mem::size_of::<Elf32Fhdr>();

        let mut header: Elf32Fhdr = make_header(1);
        header.e_phentsize = 1; // Invalid size.

        let mut image: Vec<u8> = vec![0; header_size];
        unsafe {
            write_struct(&mut image[..], &header);
        }

        let result: Result<MemoryFootprint> = memory_footprint(&image);

        assert!(result.is_err());
    }

    /// Tests that memory_footprint rejects when program header table exceeds buffer.
    #[test]
    fn memory_footprint_rejects_phdr_exceeds_buffer() {
        let header_size: usize = mem::size_of::<Elf32Fhdr>();

        let header: Elf32Fhdr = make_header(10); // Claims 10 segments but buffer is too small.

        // Buffer only has header, no room for program headers.
        let mut image: Vec<u8> = vec![0; header_size];
        unsafe {
            write_struct(&mut image[..], &header);
        }

        let result: Result<MemoryFootprint> = memory_footprint(&image);

        assert!(result.is_err());
    }

    /// Tests that memory_footprint rejects when no loadable segments are found.
    #[test]
    fn memory_footprint_rejects_no_loadable_segments() {
        let header_size: usize = mem::size_of::<Elf32Fhdr>();
        let phdr_size: usize = mem::size_of::<Elf32Phdr>();

        let header: Elf32Fhdr = make_header(1);

        // Non-loadable segment (PT_NOTE instead of PT_LOAD).
        let phdr: Elf32Phdr = Elf32Phdr {
            p_type: PT_NOTE,
            p_offset: 0,
            p_vaddr: to_u32(VADDR),
            p_paddr: 0,
            p_filesz: 0x100,
            p_memsz: 0x100,
            p_flags: PF_R,
            p_align: 1,
        };

        let mut image: Vec<u8> = vec![0; header_size + phdr_size];
        unsafe {
            write_struct(&mut image[..header_size], &header);
            write_struct(&mut image[header_size..], &phdr);
        }

        let result: Result<MemoryFootprint> = memory_footprint(&image);

        assert!(result.is_err());
    }

    /// Tests that memory_footprint correctly skips non-loadable segments.
    #[test]
    fn memory_footprint_skips_non_loadable_segments() -> Result<()> {
        let header_size: usize = mem::size_of::<Elf32Fhdr>();
        let phdr_size: usize = mem::size_of::<Elf32Phdr>();

        let header: Elf32Fhdr = make_header(3);

        // Non-loadable segment (should be skipped).
        let phdr1: Elf32Phdr = Elf32Phdr {
            p_type: PT_NOTE,
            p_offset: 0,
            p_vaddr: 0x500,
            p_paddr: 0,
            p_filesz: 0x100,
            p_memsz: 0x100,
            p_flags: PF_R,
            p_align: 1,
        };

        // Loadable segment.
        let phdr2: Elf32Phdr = Elf32Phdr {
            p_type: PT_LOAD,
            p_offset: 0,
            p_vaddr: 0x1000,
            p_paddr: 0,
            p_filesz: 0x200,
            p_memsz: 0x300,
            p_flags: PF_R | PF_X,
            p_align: 1,
        };

        // Another non-loadable segment (should be skipped).
        let phdr3: Elf32Phdr = Elf32Phdr {
            p_type: PT_DYNAMIC,
            p_offset: 0,
            p_vaddr: 0x5000,
            p_paddr: 0,
            p_filesz: 0x50,
            p_memsz: 0x50,
            p_flags: PF_R | PF_W,
            p_align: 1,
        };

        let mut image: Vec<u8> = vec![0; header_size + 3 * phdr_size];
        unsafe {
            write_struct(&mut image[..header_size], &header);
            write_struct(&mut image[header_size..header_size + phdr_size], &phdr1);
            write_struct(&mut image[header_size + phdr_size..header_size + 2 * phdr_size], &phdr2);
            write_struct(&mut image[header_size + 2 * phdr_size..], &phdr3);
        }

        let footprint: MemoryFootprint = memory_footprint(&image)?;

        // Should only consider the PT_LOAD segment (phdr2).
        assert_eq!(footprint.start(), 0x1000);
        assert_eq!(footprint.end(), 0x1300);
        assert_eq!(footprint.size(), 0x300);

        Ok(())
    }
}
