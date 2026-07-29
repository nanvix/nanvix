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
mod tests {
    use super::*;
    use ::elf::elf32::{
        EI_NIDENT,
        ELFCLASS32,
        ELFCLASS64,
        ELFDATA2LSB,
        ELFMAG0,
        ELFMAG1,
        ELFMAG2,
        ELFMAG3,
        EV_CURRENT,
        PF_R,
        PF_W,
        PF_X,
        PT_DYNAMIC,
        PT_LOAD,
        PT_NOTE,
    };
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
        ident[1] = ELFMAG1;
        ident[2] = ELFMAG2;
        ident[3] = ELFMAG3;
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
    fn load_copies_segment_and_handles_bss() -> Result<()> {
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

        // The `load()` safety contract requires zero-filled destination memory when the
        // `nightly-performance-optimizations` feature is enabled (the loader skips zeroing the
        // BSS region and relies on the caller providing zeroed memory). Honor that precondition
        // here. Without the feature, pre-fill with a sentinel (`0xaa`) so the test can verify that
        // the loader explicitly zeroes the BSS region.
        #[cfg(not(feature = "nightly-performance-optimizations"))]
        let mut memory: Vec<u8> = vec![0xaa; VADDR + memsz + 0x1000];
        #[cfg(feature = "nightly-performance-optimizations")]
        let mut memory: Vec<u8> = vec![0; VADDR + memsz + 0x1000];
        let destination: *mut c_void = memory.as_mut_ptr().cast::<c_void>();

        let (entry, first_address, size): (usize, usize, usize) =
            unsafe { load(destination, image.as_ptr(), memory.len())? };

        assert_eq!(entry, ENTRY_POINT as usize);
        assert_eq!(first_address, VADDR);
        assert_eq!(size, memsz);
        assert_eq!(&memory[VADDR..VADDR + filesz], &segment_data);

        // The loaded image must expose a zero-filled BSS region (`p_filesz..p_memsz`). By default
        // the loader zeroes it explicitly; with the `nightly-performance-optimizations` feature it
        // skips that write and relies on the caller-provided zeroed memory. Either way the range
        // must read as zero.
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
