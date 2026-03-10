// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::elf::elf32::{
    Elf32Dyn,
    Elf32Ehdr,
    Elf32Phdr,
    Elf32Rel,
    DT_JMPREL,
    DT_NULL,
    DT_PLTRELSZ,
    DT_REL,
    DT_RELSZ,
    ET_DYN,
    PT_DYNAMIC,
    PT_LOAD,
    R_386_RELATIVE,
};

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
