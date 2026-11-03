// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # ELF File Parser
//!
//! This module provides a simple parser for ELF files.
//!

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::core::ptr;
use ::elf::{
    elf32::{
        EM_386,
        ET_EXEC,
        Elf32Fhdr,
        Elf32Phdr,
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

/// ELF identification index of the file class byte (`EI_CLASS`).
const EI_CLASS: usize = 4;
/// 64-bit object file class (`ELFCLASS64`).
const ELFCLASS64: u8 = 2;

/// Returns `true` if the ELF identification bytes select the 64-bit class.
fn is_elf64(source: &[u8]) -> bool {
    source.len() > EI_CLASS && source[EI_CLASS] == ELFCLASS64
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

    // Dispatch to the 64-bit parser for ELFCLASS64 images.
    if is_elf64(source) {
        return memory_footprint64(source);
    }

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

    // Validate ELF header.
    if let Err(reason) = ehdr.validate() {
        error!("memory_footprint(): {reason}");
        return Err(anyhow::anyhow!(reason));
    }

    let phoff: usize = ehdr.e_phoff as usize;
    let phentsize: usize = ehdr.e_phentsize as usize;
    let phnum: usize = ehdr.e_phnum as usize;

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
        if phdr.is_loadable() {
            // Validate segment invariants.
            if let Err(reason) = phdr.validate() {
                error!("memory_footprint(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }

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
/// - When the `nightly-performance-optimizations` feature is enabled, the `destination` memory
///   is zero-filled. With that feature this function copies only `p_filesz` bytes per segment and
///   does not zero the trailing BSS region (`p_memsz > p_filesz`); the caller must therefore
///   guarantee that the destination memory is already zero. Without the feature the BSS region is
///   zeroed explicitly and no such guarantee is required.
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

    // Dispatch to the 64-bit loader for ELFCLASS64 images.
    if *source.add(EI_CLASS) == ELFCLASS64 {
        return load64(destination, source, max_offset);
    }

    // Get entry point (read header possibly unaligned).
    let ehdr: Elf32Fhdr = unsafe { ptr::read_unaligned(source.cast::<Elf32Fhdr>()) };

    let entry: usize = ehdr.e_entry as usize;
    trace!("entry point: {entry:#010x}");

    // Validate ELF header.
    if let Err(reason) = ehdr.validate() {
        error!("load(): {reason}");
        return Err(anyhow::anyhow!(reason));
    }

    // Check ELF type.
    if ehdr.e_type != ET_EXEC {
        let reason: &str = "invalid elf type";
        error!("load(): {reason} (e_type={})", ehdr.e_type);
        return Err(anyhow::anyhow!(reason));
    }

    // Check ELF machine architecture.
    if ehdr.e_machine != EM_386 {
        let reason: &str = "invalid machine architecture";
        error!("load(): {reason} (e_machine={})", ehdr.e_machine);
        return Err(anyhow::anyhow!(reason));
    }

    // Get program header table.
    let phdr: *const Elf32Phdr = (source as usize + ehdr.e_phoff as usize) as *const Elf32Phdr;

    // Load program segments.
    let mut loaded_segment: bool = false;
    for i in 0..ehdr.e_phnum {
        let phdr: Elf32Phdr = unsafe { ptr::read_unaligned(phdr.add(i as usize)) };

        // Loadable segment.
        if phdr.is_loadable() {
            // Check if the segment is not valid.
            if let Err(reason) = phdr.validate() {
                error!("load(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }

            let offset: usize = phdr.p_offset as usize;
            let vaddr: usize = phdr.p_vaddr as usize;
            let filesz: usize = phdr.p_filesz as usize;
            let memsz: usize = phdr.p_memsz as usize;

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
            let src: *const u8 = source.add(offset);
            let dst: *mut u8 = destination.cast::<u8>();
            let dst: *mut u8 = dst.add(vaddr);
            std::ptr::copy_nonoverlapping(src, dst, filesz);

            // Zero out the BSS section (`p_memsz > p_filesz`).
            //
            // When the `nightly-performance-optimizations` feature is enabled this is skipped:
            // the caller guarantees that the destination memory is already zero-filled (both
            // supported VMM backends allocate zero-filled guest physical memory), so the
            // trailing range is already zero and the explicit write would be redundant.
            #[cfg(not(feature = "nightly-performance-optimizations"))]
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
        error!("load(): {reason} (e_phnum={})", ehdr.e_phnum);
        return Err(anyhow::anyhow!(reason));
    }

    let size: usize = last_address - first_address;

    Ok((entry, first_address, size))
}

///
/// # Description
///
/// Computes the memory footprint of a 64-bit (ELFCLASS64) ELF file.
///
fn memory_footprint64(source: &[u8]) -> Result<MemoryFootprint> {
    let source_len: usize = source.len();
    let fh_size: usize = mem::size_of::<Elf64Fhdr>();

    if source_len < fh_size {
        let reason: &str = "buffer too small for ELF header";
        error!("memory_footprint64(): {reason} (len={source_len})");
        return Err(anyhow::anyhow!(reason));
    }

    // Safety: the buffer size check above guarantees enough bytes for the header.
    let ehdr: Elf64Fhdr = unsafe { ptr::read_unaligned(source.as_ptr().cast::<Elf64Fhdr>()) };

    if let Err(reason) = ehdr.validate() {
        error!("memory_footprint64(): {reason}");
        return Err(anyhow::anyhow!(reason));
    }

    let phoff: usize = usize::try_from(ehdr.e_phoff)
        .map_err(|_| anyhow::anyhow!("program header offset does not fit in usize"))?;
    let phentsize: usize = ehdr.e_phentsize as usize;
    let phnum: usize = ehdr.e_phnum as usize;

    let ph_table_size: usize = phentsize.checked_mul(phnum).ok_or_else(|| {
        let reason: &str = "program header table size overflow";
        anyhow::anyhow!(reason)
    })?;
    let ph_table_end: usize = phoff.checked_add(ph_table_size).ok_or_else(|| {
        let reason: &str = "program header table offset overflow";
        anyhow::anyhow!(reason)
    })?;
    if ph_table_end > source_len {
        let reason: &str = "program header table exceeds buffer";
        error!("memory_footprint64(): {reason}");
        return Err(anyhow::anyhow!(reason));
    }

    let mut end_address: usize = 0;
    let mut start_address: usize = usize::MAX;
    let mut found_loadable: bool = false;

    for i in 0..phnum {
        let entry_offset: usize = phoff + (i * phentsize);
        let entry_end: usize = entry_offset + phentsize;
        if entry_end > source_len {
            let reason: &str = "program header entry exceeds buffer";
            error!("memory_footprint64(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        // Safety: the bounds check above keeps the read within the buffer.
        let phdr_ptr: *const Elf64Phdr =
            unsafe { source.as_ptr().add(entry_offset) }.cast::<Elf64Phdr>();
        let phdr: Elf64Phdr = unsafe { ptr::read_unaligned(phdr_ptr) };

        if phdr.is_loadable() {
            if let Err(reason) = phdr.validate() {
                error!("memory_footprint64(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }

            let vaddr: usize = usize::try_from(phdr.p_vaddr)
                .map_err(|_| anyhow::anyhow!("segment vaddr does not fit in usize"))?;
            let memsz: usize = usize::try_from(phdr.p_memsz)
                .map_err(|_| anyhow::anyhow!("segment memsz does not fit in usize"))?;
            let segment_end: usize = vaddr
                .checked_add(memsz)
                .ok_or_else(|| anyhow::anyhow!("segment end address overflow"))?;

            if vaddr < start_address {
                start_address = vaddr;
            }
            if segment_end > end_address {
                end_address = segment_end;
            }
            found_loadable = true;
        }
    }

    if !found_loadable {
        let reason: &str = "no loadable segments found";
        error!("memory_footprint64(): {reason}");
        return Err(anyhow::anyhow!(reason));
    }

    debug!("memory_footprint64(): start={start_address:#010x}, end={end_address:#010x}");

    Ok(MemoryFootprint {
        start: start_address,
        end: end_address,
    })
}

///
/// # Description
///
/// Loads a 64-bit (ELFCLASS64) ELF file into memory. Mirrors [`load`] for `Elf64` headers.
///
/// # Safety
///
/// Same contract as [`load`].
///
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn load64(
    destination: *mut std::ffi::c_void,
    source: *const u8,
    max_offset: usize,
) -> Result<(usize, usize, usize)> {
    let mut first_address: usize = usize::MAX;
    let mut last_address: usize = 0;

    let ehdr: Elf64Fhdr = ptr::read_unaligned(source.cast::<Elf64Fhdr>());

    let entry: usize = usize::try_from(ehdr.e_entry)
        .map_err(|_| anyhow::anyhow!("entry point does not fit in usize"))?;
    trace!("entry point: {entry:#010x}");

    if let Err(reason) = ehdr.validate() {
        error!("load64(): {reason}");
        return Err(anyhow::anyhow!(reason));
    }

    if ehdr.e_type != ET_EXEC {
        let reason: &str = "invalid elf type";
        error!("load64(): {reason} (e_type={})", ehdr.e_type);
        return Err(anyhow::anyhow!(reason));
    }

    if ehdr.e_machine != EM_X86_64 {
        let reason: &str = "invalid machine architecture";
        error!("load64(): {reason} (e_machine={})", ehdr.e_machine);
        return Err(anyhow::anyhow!(reason));
    }

    let phoff: usize = usize::try_from(ehdr.e_phoff)
        .map_err(|_| anyhow::anyhow!("program header offset does not fit in usize"))?;
    let phdr_base: *const Elf64Phdr = (source as usize + phoff) as *const Elf64Phdr;

    let mut loaded_segment: bool = false;
    for i in 0..ehdr.e_phnum {
        let phdr: Elf64Phdr = ptr::read_unaligned(phdr_base.add(i as usize));

        if phdr.is_loadable() {
            if let Err(reason) = phdr.validate() {
                error!("load64(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }

            let offset: usize = usize::try_from(phdr.p_offset)
                .map_err(|_| anyhow::anyhow!("segment offset does not fit in usize"))?;
            let vaddr: usize = usize::try_from(phdr.p_vaddr)
                .map_err(|_| anyhow::anyhow!("segment vaddr does not fit in usize"))?;
            let filesz: usize = usize::try_from(phdr.p_filesz)
                .map_err(|_| anyhow::anyhow!("segment filesz does not fit in usize"))?;
            let memsz: usize = usize::try_from(phdr.p_memsz)
                .map_err(|_| anyhow::anyhow!("segment memsz does not fit in usize"))?;

            if vaddr + memsz > max_offset {
                let reason: String = "segment does not fit in memory".to_string();
                error!(
                    "load64(): {reason} (vaddr={vaddr:#010x}, memsz={memsz:#010x}, \
                     max_offset={max_offset:#010x})",
                );
                return Err(anyhow::anyhow!(reason));
            }

            debug!(
                "load64(): loading segment: offset={offset:#010x} vaddr={vaddr:#010x} \
                 filesz={filesz:#010x} memsz={memsz:#010x}",
            );

            let src: *const u8 = source.add(offset);
            let dst: *mut u8 = destination.cast::<u8>().add(vaddr);
            std::ptr::copy_nonoverlapping(src, dst, filesz);

            #[cfg(not(feature = "nightly-performance-optimizations"))]
            if memsz > filesz {
                std::ptr::write_bytes(dst.add(filesz), 0, memsz - filesz);
            }

            if !loaded_segment || vaddr < first_address {
                first_address = vaddr;
            }
            if vaddr + memsz > last_address {
                last_address = vaddr + memsz;
            }
            loaded_segment = true;
        }
    }

    if !loaded_segment {
        let reason: String = "no loadable segments found".to_string();
        error!("load64(): {reason} (e_phnum={})", ehdr.e_phnum);
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
mod tests;
