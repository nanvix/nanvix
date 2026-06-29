// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::elf::{
    elf32::{
        Elf32Dyn,
        Elf32Ehdr,
        Elf32Phdr,
        Elf32Rel,
        DT_JMPREL,
        DT_NULL,
        DT_PLTRELSZ,
        DT_REL,
        DT_RELSZ,
        EI_CLASS,
        ELFCLASS64,
        ET_DYN,
        PT_DYNAMIC,
        PT_LOAD,
        R_386_RELATIVE,
    },
    elf64::{
        Elf64Dyn,
        Elf64Ehdr,
        Elf64Phdr,
        Elf64Rela,
        DT_JMPREL as DT_JMPREL_64,
        DT_NULL as DT_NULL_64,
        DT_PLTREL as DT_PLTREL_64,
        DT_PLTRELSZ as DT_PLTRELSZ_64,
        DT_RELA as DT_RELA_64,
        DT_RELASZ as DT_RELASZ_64,
        R_X86_64_RELATIVE,
    },
};

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Applies relative relocations to the main PIE (`ET_DYN`) executable at CRT startup. This must be
/// called before any global data access (i.e., before `init()`).
///
/// Dispatches on the ELF identification class byte: ELFCLASS32 binaries are relocated through the
/// 32-bit `R_386_RELATIVE` (`DT_REL`) path, and ELFCLASS64 binaries through the 64-bit
/// `R_X86_64_RELATIVE` (`DT_RELA`) path.
///
/// If the binary is not `ET_DYN`, has no `PT_DYNAMIC` segment, or was loaded at its link-time
/// address (delta = 0), this function returns without modification.
///
/// # Note
///
/// This pass intentionally handles only the symbol-less `R_386_RELATIVE` fixups, which is all
/// that can be done this early: it runs from `nvx-crt0::_start` before the heap, VFS, and dlfcn
/// runtime are available. Symbol-based GOT/PLT relocations (`R_386_GLOB_DAT` / `R_386_JMP_SLOT`)
/// — including those bound against `DT_NEEDED` shared libraries — are resolved later by the
/// dlfcn self-linker (`syscall::dlfcn::dllink_executable`), which libposix invokes from
/// `__nanvix_libc_start_main` once the heap is up and before any application code runs.
///
/// # Parameters
///
/// - `base_address`: The address at which the PIE binary was loaded. The ELF header must be
///   accessible at this address.
///
/// # Safety
///
/// The caller must ensure `base_address` points to a valid, fully-loaded ELF binary whose program
/// headers and dynamic section are accessible in memory.
///
pub unsafe fn relocate_pie_binary(base_address: usize) {
    // The EI_CLASS byte is at the same offset in both ELF32 and ELF64 identification arrays.
    let class: u8 = *((base_address + EI_CLASS) as *const u8);
    if class == ELFCLASS64 {
        relocate_pie_binary_64(base_address);
    } else {
        relocate_pie_binary_32(base_address);
    }
}

///
/// # Description
///
/// Applies R_386_RELATIVE relocations to a 32-bit PIE (ET_DYN) executable.
///
/// # Safety
///
/// See [`relocate_pie_binary`].
///
unsafe fn relocate_pie_binary_32(base_address: usize) {
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

///
/// # Description
///
/// Applies `R_X86_64_RELATIVE` relocations to a 64-bit PIE (`ET_DYN`) executable. The x86-64 ABI
/// uses RELA relocations (with an explicit addend), so the relocated value is `delta + r_addend`
/// rather than the in-place-addend form used by the 32-bit `REL` path.
///
/// # Safety
///
/// See [`relocate_pie_binary`].
///
unsafe fn relocate_pie_binary_64(base_address: usize) {
    let ehdr: &Elf64Ehdr = &*(base_address as *const Elf64Ehdr);

    if ehdr.e_type != ET_DYN {
        return;
    }

    let phdrs: &[Elf64Phdr] = core::slice::from_raw_parts(
        (base_address + ehdr.e_phoff as usize) as *const Elf64Phdr,
        ehdr.e_phnum as usize,
    );

    // Find the lowest PT_LOAD p_vaddr (link-time base address).
    let link_base: u64 = phdrs
        .iter()
        .filter(|p| p.p_type == PT_LOAD)
        .map(|p| p.p_vaddr)
        .min()
        .unwrap_or(0);

    // Compute relocation delta (the load bias).
    let delta: u64 = (base_address as u64).wrapping_sub(link_base);
    if delta == 0 {
        return;
    }

    // Find PT_DYNAMIC segment.
    let dyn_phdr: &Elf64Phdr = match phdrs.iter().find(|p| p.p_type == PT_DYNAMIC) {
        Some(p) => p,
        None => return,
    };

    // Access dynamic entries at their relocated address.
    let dyn_addr: usize = dyn_phdr.p_vaddr as usize + delta as usize;
    let dyn_count: usize = dyn_phdr.p_memsz as usize / core::mem::size_of::<Elf64Dyn>();
    let dyn_entries: &[Elf64Dyn] =
        core::slice::from_raw_parts(dyn_addr as *const Elf64Dyn, dyn_count);

    // Parse relocation table addresses from dynamic entries.
    let mut dt_rela: Option<u64> = None;
    let mut dt_relasz: Option<u64> = None;
    let mut dt_jmprel: Option<u64> = None;
    let mut dt_pltrelsz: Option<u64> = None;
    let mut dt_pltrel: Option<u64> = None;

    for entry in dyn_entries {
        match entry.d_tag {
            DT_NULL_64 => break,
            DT_RELA_64 => dt_rela = Some(entry.d_val),
            DT_RELASZ_64 => dt_relasz = Some(entry.d_val),
            DT_JMPREL_64 => dt_jmprel = Some(entry.d_val),
            DT_PLTRELSZ_64 => dt_pltrelsz = Some(entry.d_val),
            DT_PLTREL_64 => dt_pltrel = Some(entry.d_val),
            _ => {},
        }
    }

    // Apply R_X86_64_RELATIVE relocations from .rela.dyn.
    if let (Some(vaddr), Some(size)) = (dt_rela, dt_relasz) {
        apply_rela_relocations_64(vaddr, size, delta);
    }

    // Apply relocations from .rela.plt, but only when the PLT relocation type is RELA (it always is
    // on x86-64). This guards against a malformed DT_PLTREL claiming REL entries.
    if dt_pltrel == Some(DT_RELA_64 as u64) {
        if let (Some(vaddr), Some(size)) = (dt_jmprel, dt_pltrelsz) {
            apply_rela_relocations_64(vaddr, size, delta);
        }
    }
}

///
/// # Description
///
/// Applies `R_X86_64_RELATIVE` fixups from a RELA relocation table. Non-`R_X86_64_RELATIVE`
/// entries are skipped.
///
/// # Parameters
///
/// - `rela_vaddr`: Link-time virtual address of the relocation table.
/// - `rela_size`: Size of the relocation table in bytes.
/// - `delta`: Relocation delta (actual load address minus link-time base).
///
/// # Safety
///
/// The caller must ensure that `rela_vaddr + delta` points to a valid relocation table in memory,
/// and that all relocation targets are writable.
///
unsafe fn apply_rela_relocations_64(rela_vaddr: u64, rela_size: u64, delta: u64) {
    let rela_addr: usize = rela_vaddr as usize + delta as usize;
    let rela_count: usize = rela_size as usize / core::mem::size_of::<Elf64Rela>();
    let relas: &[Elf64Rela] =
        core::slice::from_raw_parts(rela_addr as *const Elf64Rela, rela_count);

    for rela in relas {
        // The relocation type is the low 32 bits of r_info.
        if (rela.r_info & 0xffff_ffff) == R_X86_64_RELATIVE as u64 {
            let target: *mut u64 = (rela.r_offset as usize + delta as usize) as *mut u64;
            // RELA semantics: the relocated value is `base (delta) + addend`.
            target.write_unaligned(delta.wrapping_add(rela.r_addend as u64));
        }
    }
}
