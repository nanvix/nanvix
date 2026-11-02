// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
