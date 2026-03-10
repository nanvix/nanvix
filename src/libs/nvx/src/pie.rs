// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(dead_code)]

//==================================================================================================
// Constants
//==================================================================================================

// ELF object file types.
const ET_DYN: u16 = 3;

// ELF segment types.
const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;

// Dynamic section tags.
const DT_NULL: i32 = 0;
const DT_REL: i32 = 17;
const DT_RELSZ: i32 = 18;
const DT_JMPREL: i32 = 23;
const DT_PLTRELSZ: i32 = 2;

// Relocation types.
const R_386_RELATIVE: u32 = 8;

//==================================================================================================
// ELF Structures
//==================================================================================================

const EI_NIDENT: usize = 16;

// ELF32 file header.
#[repr(C)]
struct Elf32Ehdr {
    e_ident: [u8; EI_NIDENT], // ELF magic numbers and other info.
    e_type: u16,              // Object file type.
    e_machine: u16,           // Required machine architecture type.
    e_version: u32,           // Object file version.
    e_entry: u32,             // Virtual address of process's entry point.
    e_phoff: u32,             // Program header table file offset.
    e_shoff: u32,             // Section header table file offset.
    e_flags: u32,             // Processor-specific flags.
    e_ehsize: u16,            // ELF header's size in bytes.
    e_phentsize: u16,         // Program header table entry size.
    e_phnum: u16,             // Entries in the program header table.
    e_shentsize: u16,         // Section header table size.
    e_shnum: u16,             // Entries in the section header table.
    e_shstrndx: u16,          // Index for the section name string table.
}

// ELF32 program header.
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

// ELF32 dynamic section entry.
#[repr(C)]
struct Elf32Dyn {
    d_tag: i32, // Entry type tag.
    d_val: u32, // Integer value.
}

// ELF32 relocation entry (without addend).
#[repr(C)]
struct Elf32Rel {
    r_offset: u32, // Offset at which to apply the relocation.
    r_info: u32,   // Relocation type and symbol index.
}

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Applies R_386_RELATIVE relocations to the main PIE (ET_DYN) executable at CRT startup. This
/// must be called before any global data access (i.e., before `init()`).
///
/// If the binary is not ET_DYN, has no PT_DYNAMIC segment, or was loaded at its link-time
/// address (delta = 0), this function returns without modification.
///
/// # Parameters
///
/// - `base_address`: The address at which the PIE binary was loaded. The ELF header must be
///   accessible at this address.
///
/// # Safety
///
/// The caller must ensure `base_address` points to a valid, fully-loaded ELF32 binary whose
/// program headers and dynamic section are accessible in memory.
///
pub unsafe fn relocate_pie_binary(base_address: usize) {
    let ehdr: &Elf32Ehdr = &*(base_address as *const Elf32Ehdr);

    if ehdr.e_type != ET_DYN {
        return;
    }

    let phdrs: &[Elf32Phdr] = core::slice::from_raw_parts(
        (base_address + ehdr.e_phoff as usize) as *const Elf32Phdr,
        ehdr.e_phnum as usize,
    );

    // Find the lowest PT_LOAD p_vaddr (link-time base address).
    let link_base: u32 = phdrs
        .iter()
        .filter(|p| p.p_type == PT_LOAD)
        .map(|p| p.p_vaddr)
        .min()
        .unwrap_or(0);

    // Compute relocation delta.
    let delta: u32 = (base_address as u32).wrapping_sub(link_base);
    if delta == 0 {
        return;
    }

    // Find PT_DYNAMIC segment.
    let dyn_phdr: &Elf32Phdr = match phdrs.iter().find(|p| p.p_type == PT_DYNAMIC) {
        Some(p) => p,
        None => return,
    };

    // Access dynamic entries at their relocated address.
    let dyn_addr: usize = dyn_phdr.p_vaddr as usize + delta as usize;
    let dyn_count: usize = dyn_phdr.p_memsz as usize / core::mem::size_of::<Elf32Dyn>();
    let dyn_entries: &[Elf32Dyn] =
        core::slice::from_raw_parts(dyn_addr as *const Elf32Dyn, dyn_count);

    // Parse relocation table addresses from dynamic entries.
    let mut dt_rel: Option<u32> = None;
    let mut dt_relsz: Option<u32> = None;
    let mut dt_jmprel: Option<u32> = None;
    let mut dt_pltrelsz: Option<u32> = None;

    for entry in dyn_entries {
        match entry.d_tag {
            DT_NULL => break,
            DT_REL => dt_rel = Some(entry.d_val),
            DT_RELSZ => dt_relsz = Some(entry.d_val),
            DT_JMPREL => dt_jmprel = Some(entry.d_val),
            DT_PLTRELSZ => dt_pltrelsz = Some(entry.d_val),
            _ => {},
        }
    }

    // Apply R_386_RELATIVE relocations from .rel.dyn.
    if let (Some(vaddr), Some(size)) = (dt_rel, dt_relsz) {
        apply_relative_relocations(vaddr, size, delta);
    }

    // Apply R_386_RELATIVE relocations from .rel.plt.
    if let (Some(vaddr), Some(size)) = (dt_jmprel, dt_pltrelsz) {
        apply_relative_relocations(vaddr, size, delta);
    }
}

///
/// # Description
///
/// Applies R_386_RELATIVE fixups from a relocation table. Non-R_386_RELATIVE entries are
/// skipped.
///
/// # Parameters
///
/// - `rel_vaddr`: Link-time virtual address of the relocation table.
/// - `rel_size`: Size of the relocation table in bytes.
/// - `delta`: Relocation delta (actual load address minus link-time base).
///
/// # Safety
///
/// The caller must ensure that `rel_vaddr + delta` points to a valid relocation table in
/// memory, and that all relocation targets are writable.
///
unsafe fn apply_relative_relocations(rel_vaddr: u32, rel_size: u32, delta: u32) {
    let rel_addr: usize = rel_vaddr as usize + delta as usize;
    let rel_count: usize = rel_size as usize / core::mem::size_of::<Elf32Rel>();
    let rels: &[Elf32Rel] = core::slice::from_raw_parts(rel_addr as *const Elf32Rel, rel_count);

    for rel in rels {
        if (rel.r_info & 0xff) == R_386_RELATIVE {
            let target: *mut u32 = (rel.r_offset as usize + delta as usize) as *mut u32;
            let addend: u32 = target.read_unaligned();
            target.write_unaligned(addend.wrapping_add(delta));
        }
    }
}
