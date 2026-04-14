// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # ELF64 File Support
//!
//! This module provides ELF64 structures and loading functions for 64-bit binaries.
//!

//==================================================================================================
// Imports
//==================================================================================================

use super::MemoryFootprint;
use ::anyhow::Result;
use ::core::ptr;
use ::elf::{
    elf32::{
        ELFDATA2LSB,
        ELFMAG0,
        ELFMAG1,
        ELFMAG2,
        ELFMAG3,
        ET_EXEC,
        EV_CURRENT,
        PT_LOAD,
    },
    elf64::{
        EM_X86_64,
        Elf64Fhdr,
        Elf64Phdr,
    },
};
use ::log::{
    debug,
    error,
    trace,
};
use ::std::mem;

//==================================================================================================
// Functions
//==================================================================================================

/// Computes the memory footprint of a 64-bit ELF file.
pub(super) fn memory_footprint_64(source: &[u8]) -> Result<MemoryFootprint> {
    let source_len: usize = source.len();
    let fh_size: usize = mem::size_of::<Elf64Fhdr>();

    // Check if buffer is too small to contain ELF header.
    if source_len < fh_size {
        let reason: &str = "buffer too small for ELF header";
        error!("memory_footprint(): {reason} (len={source_len})");
        return Err(anyhow::anyhow!(reason));
    }

    // Safety: the buffer size check above guarantees that `source` contains at least
    // `size_of::<Elf64Fhdr>()` bytes, so the read is within bounds.
    let ehdr: Elf64Fhdr = unsafe { ptr::read_unaligned(source.as_ptr().cast::<Elf64Fhdr>()) };

    // Check if ELF magic number is valid.
    if !ehdr.is_valid() {
        let reason: &str = "header is null or invalid magic";
        return Err(anyhow::anyhow!(reason));
    }

    let phoff: usize =
        usize::try_from(ehdr.e_phoff).map_err(|_| anyhow::anyhow!("e_phoff overflows usize"))?;
    let phentsize: usize = ehdr.e_phentsize as usize;
    let phnum: usize = ehdr.e_phnum as usize;

    // Check if program header has an invalid size.
    if phentsize != mem::size_of::<Elf64Phdr>() {
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
        let phdr_ptr: *const Elf64Phdr =
            unsafe { source.as_ptr().add(entry_offset) }.cast::<Elf64Phdr>();
        let phdr: Elf64Phdr = unsafe { ptr::read_unaligned(phdr_ptr) };

        // Loadable segment.
        if phdr.p_type == PT_LOAD {
            let vaddr: usize = usize::try_from(phdr.p_vaddr)
                .map_err(|_| anyhow::anyhow!("p_vaddr overflows usize"))?;
            let memsz: usize = usize::try_from(phdr.p_memsz)
                .map_err(|_| anyhow::anyhow!("p_memsz overflows usize"))?;
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

/// Loads a 64-bit ELF file into memory.
#[allow(unsafe_op_in_unsafe_fn)]
pub(super) unsafe fn load_64(
    destination: *mut std::ffi::c_void,
    source: *const u8,
    max_offset: usize,
) -> Result<(usize, usize, usize)> {
    let mut first_address: usize = usize::MAX;
    let mut last_address: usize = 0;

    // Get entry point.
    let ehdr: *const Elf64Fhdr = source.cast::<Elf64Fhdr>();

    let entry: usize =
        usize::try_from((*ehdr).e_entry).map_err(|_| anyhow::anyhow!("e_entry overflows usize"))?;
    trace!("entry point: {entry:#010x}");

    // Check if ELF magic number is valid.
    if (*ehdr).e_ident[0] != ELFMAG0
        || (*ehdr).e_ident[1] != ELFMAG1
        || (*ehdr).e_ident[2] != ELFMAG2
        || (*ehdr).e_ident[3] != ELFMAG3
    {
        let reason: String = "header is null or invalid magic".to_string();
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
    if (*ehdr).e_machine != EM_X86_64 {
        let reason: String = "invalid machine architecture".to_string();
        error!("load(): {reason} (e_machine={})", (*ehdr).e_machine);
        return Err(anyhow::anyhow!(reason));
    }

    // Get program header table.
    let phoff: usize =
        usize::try_from((*ehdr).e_phoff).map_err(|_| anyhow::anyhow!("e_phoff overflows usize"))?;
    let phdr: *const Elf64Phdr = (source as usize + phoff) as *const Elf64Phdr;

    // Check if program header has an invalid size.
    if (*ehdr).e_phentsize as usize != mem::size_of::<Elf64Phdr>() {
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
            let offset: usize = usize::try_from(phdr.p_offset)
                .map_err(|_| anyhow::anyhow!("p_offset overflows usize"))?;
            let vaddr: usize = usize::try_from(phdr.p_vaddr)
                .map_err(|_| anyhow::anyhow!("p_vaddr overflows usize"))?;
            let filesz: usize = usize::try_from(phdr.p_filesz)
                .map_err(|_| anyhow::anyhow!("p_filesz overflows usize"))?;
            let memsz: usize = usize::try_from(phdr.p_memsz)
                .map_err(|_| anyhow::anyhow!("p_memsz overflows usize"))?;

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
