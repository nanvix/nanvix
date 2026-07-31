// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::vec::Vec;
#[cfg(target_arch = "x86")]
use ::elf::elf32::{
    Elf32Dyn,
    DT_JMPREL,
    DT_NEEDED,
    DT_NULL,
    DT_PLTRELSZ,
    DT_REL,
    DT_RELSZ,
    DT_SYMTAB,
};
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use ::elf::elf64::{
    Elf64Dyn,
    DT_JMPREL,
    DT_NEEDED,
    DT_NULL,
    DT_PLTRELSZ,
    DT_RELA,
    DT_RELASZ,
    DT_SYMTAB,
};
use ::elf::{
    RelocationEntry,
    RelocationTable,
    RelocationType,
    StringTable,
    Symbol,
    SymbolTable,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
#[cfg(target_arch = "aarch64")]
use ::sys::mm::VirtualAddress;

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Binds the main executable's symbol-based relocations (`R_386_JMP_SLOT` /
/// `R_386_GLOB_DAT`, plus `R_386_32` / `R_386_PC32`) against the global symbol
/// table at process startup. This complements [`nvx::pie::relocate_pie_binary`],
/// which runs earlier (before the heap is up) and applies only the
/// symbol-less `R_386_RELATIVE` fixups.
///
/// The routine is a no-op unless the executable carries a dynamic section
/// (`.dynamic`) that declares at least one `DT_NEEDED` shared-library
/// dependency. For such images it:
///
/// 1. Loads every `DT_NEEDED` dependency into the global scope (the equivalent
///    of `dlopen(name, RTLD_GLOBAL | RTLD_NOW)`), making the dependency's
///    exported symbols resolvable.
/// 2. Walks the executable's `.rel.dyn` and `.rel.plt` tables and resolves each
///    symbol-based relocation, reusing the dlfcn resolver's symbol logic (the
///    global symbol table and the System V weak-undefined-resolves-to-zero
///    rule).
///
/// The dynamic section is located through the `__dynamic_start`/`__dynamic_end`
/// linker-script symbols rather than by parsing the ELF header: on Nanvix the
/// first `PT_LOAD` segment begins at a non-zero file offset, so the ELF header
/// and program headers are *not* mapped at the image's load address and cannot
/// be read at runtime.
///
/// This must run after the heap is available (it allocates while loading
/// dependencies) and before any application code (`.init_array` constructors or
/// `main`) executes, so the executable's GOT/PLT slots are bound before first
/// use.
///
/// # Returns
///
/// `Ok(())` on success (including the no-op cases). Returns an error if one or
/// more required (non-weak) symbols could not be resolved; the successfully
/// resolved relocations are still applied.
///
/// # Safety
///
/// Must be called once, on the main executable's own thread of control, while
/// the dynamic section, relocation tables and dynamic symbol/string tables of
/// the loaded image are mapped in memory (they always are once the image is
/// running).
///
#[cfg(target_arch = "x86")]
pub unsafe fn dllink_executable() -> Result<(), Error> {
    // Locate the dynamic array via the `__dynamic_start`/`__dynamic_end`
    // linker-script symbols. A statically linked image (no dynamic section) has
    // an empty range and there is nothing to bind.
    let dyn_entries: &[Elf32Dyn] = match dynamic_section() {
        Some(entries) => entries,
        None => return Ok(()),
    };

    // Parse the relocation-table descriptors and the DT_NEEDED list.
    let mut dt_rel: Option<u32> = None;
    let mut dt_relsz: Option<u32> = None;
    let mut dt_jmprel: Option<u32> = None;
    let mut dt_pltrelsz: Option<u32> = None;
    let mut dt_symtab: Option<u32> = None;
    let mut needed: Vec<u32> = Vec::new();

    for entry in dyn_entries {
        match entry.d_tag {
            DT_NEEDED => needed.push(entry.d_val),
            DT_REL => dt_rel = Some(entry.d_val),
            DT_RELSZ => dt_relsz = Some(entry.d_val),
            DT_JMPREL => dt_jmprel = Some(entry.d_val),
            DT_PLTRELSZ => dt_pltrelsz = Some(entry.d_val),
            DT_SYMTAB => dt_symtab = Some(entry.d_val),
            _ => {},
        }
    }

    // An executable that declares no shared-library dependencies has no
    // externally-provided symbols to bind; the earlier R_386_RELATIVE pass
    // already covered everything. Nothing to do.
    if needed.is_empty() {
        return Ok(());
    }

    // Locate the executable's own dynamic symbol/string tables (the same
    // linker-script boundaries that `dlinit()` consumes).
    let (dynsym_start, dynsym, dynstr) = match exe_dynsym_dynstr() {
        Some(tables) => tables,
        None => {
            ::syslog::warn!(
                "dllink_executable(): executable has no .dynsym/.dynstr; cannot bind symbols"
            );
            return Ok(());
        },
    };

    // Relocation delta (actual load address minus link-time base). The dynamic
    // symbol table is reachable both at its runtime address (the `__dynsym_start`
    // linker symbol) and at its link-time address (`DT_SYMTAB`); their difference
    // is the load bias. On Nanvix the main executable loads at its link base, so
    // this is zero, but computing it keeps the resolver correct regardless.
    let delta: u32 = match dt_symtab {
        Some(link_vaddr) => (dynsym_start as u32).wrapping_sub(link_vaddr),
        None => 0,
    };

    // Publish the executable's own exported symbols into the global table, then
    // load each DT_NEEDED dependency into the global scope so its exported
    // symbols become resolvable.
    super::dlinit();
    for off in &needed {
        let name: &str = match dynstr.get_name(*off as usize) {
            Ok(name) if !name.is_empty() => name,
            _ => continue,
        };
        match super::dlopen(name, true) {
            Ok(_) => ::syslog::trace!("dllink_executable(): loaded DT_NEEDED {}", name),
            Err(e) => ::syslog::warn!(
                "dllink_executable(): failed to load DT_NEEDED {} (error={:?})",
                name,
                e
            ),
        }
    }

    // Resolve the executable's symbol-based relocations from .rel.dyn and
    // .rel.plt.
    let mut unresolved: usize = 0;
    if let (Some(vaddr), Some(size)) = (dt_rel, dt_relsz) {
        unresolved += resolve_table(vaddr, size, delta, &dynsym, &dynstr);
    }
    if let (Some(vaddr), Some(size)) = (dt_jmprel, dt_pltrelsz) {
        unresolved += resolve_table(vaddr, size, delta, &dynsym, &dynstr);
    }

    if unresolved > 0 {
        ::syslog::warn!("dllink_executable(): {} unresolved executable relocations", unresolved);
        return Err(Error::new(ErrorCode::NoSuchEntry, "unresolved executable relocations"));
    }

    Ok(())
}

///
/// # Description
///
/// Runs the `.fini_array` destructors of the executable's startup-loaded
/// shared-library dependencies, in the reverse of their load order. This is the
/// process-exit counterpart of [`dllink_executable`]: that routine loads the
/// executable's `DT_NEEDED` dependencies into the global scope and runs their
/// `.preinit_array` / `.init_array` constructors (via `dlopen`) before `main`;
/// this runs their `.fini_array` destructors after `main` returns.
///
/// The startup dependencies are loaded with `RTLD_GLOBAL` and are therefore
/// pinned for the lifetime of the process — `dlclose` never unloads them — so
/// their destructors are not driven by the normal `dlclose` teardown path.
/// This routine drives them at process exit instead, newest dependency first,
/// as required by the System V gABI.
///
/// The walk covers every library currently in the global scope (the startup
/// `DT_NEEDED` dependencies plus anything later promoted with
/// `dlopen(RTLD_GLOBAL)`); libraries loaded `RTLD_LOCAL` and already torn down
/// through `dlclose` are unaffected.
///
/// # Returns
///
/// Nothing. Destructors that panic propagate as usual; a library with no
/// `.fini_array` is skipped.
///
/// # Safety
///
/// Must be called once, on the main executable's own thread of control, after
/// `main` (and the executable's own `.fini_array`) has returned and while the
/// heap is still up — a destructor may allocate or free. The destructors are
/// the loaded libraries' own trusted teardown routines. Repeated calls are a
/// no-op (guarded by a `Once`).
///
pub unsafe fn dlfini_executable() {
    super::run_global_destructors();
}

//==================================================================================================
// Private Functions
//==================================================================================================

///
/// # Description
///
/// Returns the executable's dynamic array (`.dynamic`) as a slice bounded by the
/// `__dynamic_start` / `__dynamic_end` linker-script symbols, truncated at its
/// `DT_NULL` terminator. Returns `None` when the image carries no dynamic
/// section (the boundaries collapse to an empty range, as for a statically
/// linked image).
///
/// The dynamic array lives in a loaded segment and is therefore mapped at
/// runtime, unlike the ELF header. The boundary symbols are provided by the
/// linker script (`PROVIDE`, mirroring `__dynsym_start`/`__dynstr_start`), so the
/// reference resolves in every image — empty when there is no `.dynamic`.
///
/// # Safety
///
/// The `__dynamic_*` boundary symbols must delimit a valid, in-memory
/// `.dynamic` section of the loaded executable image.
///
#[cfg(target_arch = "x86")]
unsafe fn dynamic_section() -> Option<&'static [Elf32Dyn]> {
    extern "C" {
        static __dynamic_start: u8;
        static __dynamic_end: u8;
    }

    let start: usize = &__dynamic_start as *const u8 as usize;
    let end: usize = &__dynamic_end as *const u8 as usize;

    let size: usize = end.saturating_sub(start);
    let capacity: usize = size / core::mem::size_of::<Elf32Dyn>();
    if capacity == 0 {
        return None;
    }

    // Truncate at the DT_NULL terminator (the entries past it are padding).
    let base: *const Elf32Dyn = start as *const Elf32Dyn;
    let mut len: usize = 0;
    while len < capacity {
        if (*base.add(len)).d_tag == DT_NULL {
            break;
        }
        len += 1;
    }

    Some(core::slice::from_raw_parts(base, len))
}

///
/// # Description
///
/// Returns the executable's dynamic symbol and string tables, together with the
/// runtime start address of the symbol table, bounded by the `__dynsym_*` /
/// `__dynstr_*` linker-script symbols. Returns `None` when the executable was
/// linked without `--export-dynamic` (the boundaries collapse to an empty
/// range).
///
/// # Safety
///
/// The linker-script boundary symbols must delimit valid, in-memory `.dynsym` /
/// `.dynstr` sections of the loaded executable image.
///
unsafe fn exe_dynsym_dynstr() -> Option<(usize, SymbolTable, StringTable)> {
    extern "C" {
        static __dynsym_start: u8;
        static __dynsym_end: u8;
        static __dynstr_start: u8;
        static __dynstr_end: u8;
    }

    let dynsym_start: usize = &__dynsym_start as *const u8 as usize;
    let dynsym_end: usize = &__dynsym_end as *const u8 as usize;
    let dynstr_start: usize = &__dynstr_start as *const u8 as usize;
    let dynstr_end: usize = &__dynstr_end as *const u8 as usize;

    let dynsym_size: usize = dynsym_end.saturating_sub(dynsym_start);
    let dynstr_size: usize = dynstr_end.saturating_sub(dynstr_start);
    if dynsym_size == 0 || dynstr_size == 0 {
        return None;
    }

    let sym_count: usize = dynsym_size / core::mem::size_of::<Symbol>();
    let dynsym: SymbolTable = SymbolTable::from_raw_parts(dynsym_start as *mut Symbol, sym_count);
    let dynstr: StringTable = StringTable::from_raw_parts(dynstr_start as *const u8, dynstr_size);

    Some((dynsym_start, dynsym, dynstr))
}

///
/// # Description
///
/// Resolves all symbol-based relocations in a single relocation table, returning
/// the number that could not be resolved.
///
/// # Safety
///
/// `rel_vaddr + delta` must point to a valid relocation table of `rel_size`
/// bytes whose targets are writable, and `dynsym` / `dynstr` must describe the
/// executable's dynamic symbol/string tables.
///
#[cfg(target_arch = "x86")]
unsafe fn resolve_table(
    rel_vaddr: u32,
    rel_size: u32,
    delta: u32,
    dynsym: &SymbolTable,
    dynstr: &StringTable,
) -> usize {
    let rel_addr: usize = rel_vaddr as usize + delta as usize;
    let count: usize = rel_size as usize / core::mem::size_of::<RelocationEntry>();
    let table: RelocationTable =
        RelocationTable::from_raw_parts(rel_addr as *mut RelocationEntry, count);

    let mut unresolved: usize = 0;
    for rel in table.iter() {
        let typ: RelocationType = match rel.typ() {
            Ok(typ) => typ,
            // Unknown / processor-specific relocation: leave the slot untouched.
            Err(_) => continue,
        };

        match typ {
            // Already applied by `nvx::pie::relocate_pie_binary` (a no-op when
            // the load delta is zero); nothing to bind here.
            RelocationType::R_386_RELATIVE | RelocationType::R_386_NONE => {},

            RelocationType::R_386_GLOB_DAT
            | RelocationType::R_386_JMP_SLOT
            | RelocationType::R_386_32
            | RelocationType::R_386_PC32 => {
                if !apply_symbol_relocation(rel, &typ, delta, dynsym, dynstr) {
                    unresolved += 1;
                }
            },

            other => {
                ::syslog::warn!("dllink_executable(): unsupported relocation type {:?}", other);
            },
        }
    }

    unresolved
}

///
/// # Description
///
/// Resolves a single symbol-based relocation, writing the resolved value into
/// the relocation target. Returns `false` if the referenced symbol could not be
/// resolved (and is not a weak undefined symbol, which legally resolves to
/// zero).
///
/// # Safety
///
/// `rel` must reference a valid symbol index into `dynsym`, and its target
/// (`rel.offset() + delta`) must be writable.
///
#[cfg(target_arch = "x86")]
unsafe fn apply_symbol_relocation(
    rel: &RelocationEntry,
    typ: &RelocationType,
    delta: u32,
    dynsym: &SymbolTable,
    dynstr: &StringTable,
) -> bool {
    let index: usize = rel.symbol_index() as usize;
    let sym: &Symbol = match dynsym.get(index) {
        Some(sym) => sym,
        None => {
            ::syslog::warn!("dllink_executable(): invalid symbol index {}", index);
            return false;
        },
    };

    let name: &str = match dynstr.get_name(sym.name_offset()) {
        Ok(name) => name,
        Err(_) => return false,
    };

    // Resolve the symbol to an absolute runtime address.
    let value: u32 = if !sym.is_undefined() {
        // Defined within the executable itself.
        sym.value().wrapping_add(delta)
    } else {
        match super::global_symbol_lookup(name) {
            Some(addr) => addr as u32,
            None => {
                // System V ABI: an unresolved weak undefined symbol is taken to
                // have the value zero. Every other unresolved symbol is an error.
                if sym.is_weak() {
                    0
                } else {
                    ::syslog::warn!("dllink_executable(): symbol not found: {}", name);
                    return false;
                }
            },
        }
    };

    let target: *mut u32 = (rel.offset() as usize + delta as usize) as *mut u32;
    match typ {
        // GOT/PLT slots take the symbol value directly (the implicit addend is
        // zero for these types).
        RelocationType::R_386_JMP_SLOT | RelocationType::R_386_GLOB_DAT => {
            target.write_unaligned(value);
        },
        // S + A, with wrapping 32-bit ELF arithmetic.
        RelocationType::R_386_32 => {
            let addend: u32 = target.read_unaligned();
            target.write_unaligned(value.wrapping_add(addend));
        },
        // S + A - P, with wrapping 32-bit ELF arithmetic.
        RelocationType::R_386_PC32 => {
            let addend: u32 = target.read_unaligned();
            let place: u32 = target as u32;
            target.write_unaligned(value.wrapping_add(addend).wrapping_sub(place));
        },
        // Unreachable: callers only dispatch the four types handled above.
        _ => return false,
    }

    true
}

//==================================================================================================
// x86-64 (ELF64 / RELA) self-linker
//==================================================================================================

///
/// # Description
///
/// x86-64 counterpart of [`dllink_executable`]. Binds the main executable's symbol-based
/// relocations (`R_X86_64_JUMP_SLOT` / `R_X86_64_GLOB_DAT`, plus `R_X86_64_64` / `R_X86_64_PC32`)
/// against the global symbol table at process startup, after loading each `DT_NEEDED` dependency
/// into the global scope. It mirrors the i386 path but parses the ELF64 dynamic section, reads the
/// RELA relocation tables (`DT_RELA` / `DT_JMPREL`) with their explicit addends, and writes 64-bit
/// GOT/PLT slots. The symbol-less `R_X86_64_RELATIVE` fixups are already applied earlier by
/// [`nvx::pie::relocate_pie_binary`].
///
/// # Returns
///
/// `Ok(())` on success (including the no-op cases). Returns an error if one or more required
/// (non-weak) symbols could not be resolved.
///
/// # Safety
///
/// Same contract as [`dllink_executable`]: must run once, on the main thread, after the heap is up
/// and before any application code executes.
///
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub unsafe fn dllink_executable() -> Result<(), Error> {
    // Locate the dynamic array via the `__dynamic_start`/`__dynamic_end` linker-script symbols. A
    // statically linked image (no dynamic section) has an empty range and there is nothing to bind.
    let dyn_entries: &[Elf64Dyn] = match dynamic_section() {
        Some(entries) => entries,
        None => return Ok(()),
    };

    // Parse the RELA relocation-table descriptors and the DT_NEEDED list.
    let mut dt_rela: Option<u64> = None;
    let mut dt_relasz: Option<u64> = None;
    let mut dt_jmprel: Option<u64> = None;
    let mut dt_pltrelsz: Option<u64> = None;
    let mut dt_symtab: Option<u64> = None;
    let mut needed: Vec<u64> = Vec::new();

    for entry in dyn_entries {
        match entry.d_tag {
            DT_NEEDED => needed.push(entry.d_val),
            DT_RELA => dt_rela = Some(entry.d_val),
            DT_RELASZ => dt_relasz = Some(entry.d_val),
            DT_JMPREL => dt_jmprel = Some(entry.d_val),
            DT_PLTRELSZ => dt_pltrelsz = Some(entry.d_val),
            DT_SYMTAB => dt_symtab = Some(entry.d_val),
            _ => {},
        }
    }

    // An executable that declares no shared-library dependencies has no externally-provided symbols
    // to bind; the earlier R_X86_64_RELATIVE pass already covered everything.
    if needed.is_empty() {
        return Ok(());
    }

    // Locate the executable's own dynamic symbol/string tables.
    let (dynsym_start, dynsym, dynstr) = match exe_dynsym_dynstr() {
        Some(tables) => tables,
        None => {
            ::syslog::warn!(
                "dllink_executable(): executable has no .dynsym/.dynstr; cannot bind symbols"
            );
            return Ok(());
        },
    };

    // Relocation delta (actual load address minus link-time base). On Nanvix the main executable
    // loads at its link base, so this is zero, but computing it keeps the resolver correct.
    let delta: u64 = match dt_symtab {
        Some(link_vaddr) => (dynsym_start as u64).wrapping_sub(link_vaddr),
        None => 0,
    };

    // Publish the executable's own exported symbols, then load each DT_NEEDED dependency into the
    // global scope so its exported symbols become resolvable.
    super::dlinit();
    for off in &needed {
        let name: &str = match dynstr.get_name(*off as usize) {
            Ok(name) if !name.is_empty() => name,
            _ => continue,
        };
        match super::dlopen(name, true) {
            Ok(_) => ::syslog::trace!("dllink_executable(): loaded DT_NEEDED {}", name),
            Err(e) => ::syslog::warn!(
                "dllink_executable(): failed to load DT_NEEDED {} (error={:?})",
                name,
                e
            ),
        }
    }

    // Resolve the executable's symbol-based relocations from .rela.dyn and .rela.plt.
    let mut unresolved: usize = 0;
    if let (Some(vaddr), Some(size)) = (dt_rela, dt_relasz) {
        unresolved += resolve_table(vaddr, size, delta, &dynsym, &dynstr);
    }
    if let (Some(vaddr), Some(size)) = (dt_jmprel, dt_pltrelsz) {
        unresolved += resolve_table(vaddr, size, delta, &dynsym, &dynstr);
    }

    if unresolved > 0 {
        ::syslog::warn!("dllink_executable(): {} unresolved executable relocations", unresolved);
        return Err(Error::new(ErrorCode::NoSuchEntry, "unresolved executable relocations"));
    }

    Ok(())
}

/// x86-64 counterpart of [`dynamic_section`]: returns the executable's `.dynamic` array as a slice
/// of ELF64 dynamic entries, truncated at its `DT_NULL` terminator.
///
/// # Safety
///
/// The `__dynamic_*` boundary symbols must delimit a valid, in-memory `.dynamic` section.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
unsafe fn dynamic_section() -> Option<&'static [Elf64Dyn]> {
    extern "C" {
        static __dynamic_start: u8;
        static __dynamic_end: u8;
    }

    let start: usize = &__dynamic_start as *const u8 as usize;
    let end: usize = &__dynamic_end as *const u8 as usize;

    let size: usize = end.saturating_sub(start);
    let capacity: usize = size / core::mem::size_of::<Elf64Dyn>();
    if capacity == 0 {
        return None;
    }

    // Truncate at the DT_NULL terminator (the entries past it are padding).
    let base: *const Elf64Dyn = start as *const Elf64Dyn;
    let mut len: usize = 0;
    while len < capacity {
        if (*base.add(len)).d_tag == DT_NULL {
            break;
        }
        len += 1;
    }

    Some(core::slice::from_raw_parts(base, len))
}

/// ELF64 counterpart of [`resolve_table`]: resolves all symbol-based relocations in a single RELA
/// relocation table, returning the number that could not be resolved.
///
/// # Safety
///
/// `rel_vaddr + delta` must point to a valid RELA relocation table of `rel_size` bytes whose
/// targets are writable, and `dynsym` / `dynstr` must describe the executable's dynamic tables.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
unsafe fn resolve_table(
    rel_vaddr: u64,
    rel_size: u64,
    delta: u64,
    dynsym: &SymbolTable,
    dynstr: &StringTable,
) -> usize {
    let rel_addr: usize = rel_vaddr as usize + delta as usize;
    let count: usize = rel_size as usize / core::mem::size_of::<RelocationEntry>();
    let table: RelocationTable =
        RelocationTable::from_raw_parts(rel_addr as *mut RelocationEntry, count);

    let mut unresolved: usize = 0;
    for rel in table.iter() {
        let typ: RelocationType = match rel.typ() {
            Ok(typ) => typ,
            // Unknown / processor-specific relocation: leave the slot untouched.
            Err(_) => continue,
        };

        match typ {
            // Already applied by `nvx::pie::relocate_pie_binary` (a no-op when the load delta is
            // zero); nothing to bind here.
            #[cfg(target_arch = "x86_64")]
            RelocationType::R_X86_64_RELATIVE | RelocationType::R_X86_64_NONE => {},
            #[cfg(target_arch = "aarch64")]
            RelocationType::R_AARCH64_RELATIVE | RelocationType::R_AARCH64_NONE => {},

            #[cfg(target_arch = "x86_64")]
            RelocationType::R_X86_64_GLOB_DAT
            | RelocationType::R_X86_64_JUMP_SLOT
            | RelocationType::R_X86_64_64
            | RelocationType::R_X86_64_PC32 => {
                if !apply_symbol_relocation(rel, &typ, delta, dynsym, dynstr) {
                    unresolved += 1;
                }
            },
            #[cfg(target_arch = "aarch64")]
            RelocationType::R_AARCH64_GLOB_DAT
            | RelocationType::R_AARCH64_JUMP_SLOT
            | RelocationType::R_AARCH64_ABS64
            | RelocationType::R_AARCH64_PREL64 => {
                if !apply_symbol_relocation(rel, &typ, delta, dynsym, dynstr) {
                    unresolved += 1;
                }
            },

            #[cfg(target_arch = "x86_64")]
            other => {
                ::syslog::warn!("dllink_executable(): unsupported relocation type {:?}", other);
            },
        }
    }

    unresolved
}

/// ELF64 counterpart of [`apply_symbol_relocation`]: resolves a single symbol-based RELA relocation,
/// writing the resolved value (using the entry's explicit addend) into the target.
/// Returns `false` if the referenced symbol could not be resolved and is not a weak undefined
/// symbol.
///
/// # Safety
///
/// `rel` must reference a valid symbol index into `dynsym`, and its target (`rel.offset() + delta`)
/// must be writable.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
unsafe fn apply_symbol_relocation(
    rel: &RelocationEntry,
    typ: &RelocationType,
    delta: u64,
    dynsym: &SymbolTable,
    dynstr: &StringTable,
) -> bool {
    let index: usize = rel.symbol_index() as usize;
    let sym: &Symbol = match dynsym.get(index) {
        Some(sym) => sym,
        None => {
            ::syslog::warn!("dllink_executable(): invalid symbol index {}", index);
            return false;
        },
    };

    let name: &str = match dynstr.get_name(sym.name_offset()) {
        Ok(name) => name,
        Err(_) => return false,
    };

    // Resolve the symbol to an absolute runtime address.
    let value: u64 = if !sym.is_undefined() {
        // Defined within the executable itself.
        sym.value().wrapping_add(delta)
    } else {
        match super::global_symbol_lookup(name) {
            Some(addr) => addr as u64,
            None => {
                // System V ABI: an unresolved weak undefined symbol is taken to have the value
                // zero. Every other unresolved symbol is an error.
                if sym.is_weak() {
                    0
                } else {
                    ::syslog::warn!("dllink_executable(): symbol not found: {}", name);
                    return false;
                }
            },
        }
    };

    // RELA carries an explicit addend; the relocation target is the entry offset biased by the load
    // delta.
    let addend: i64 = rel.addend();
    let place: u64 = rel.offset().wrapping_add(delta);
    match typ {
        // GOT/PLT slots take the symbol value (`S + A`, with the addend zero for these types),
        // written as a 64-bit word.
        #[cfg(target_arch = "x86_64")]
        RelocationType::R_X86_64_JUMP_SLOT
        | RelocationType::R_X86_64_GLOB_DAT
        | RelocationType::R_X86_64_64 => {
            let target: *mut u64 = place as *mut u64;
            target.write_unaligned(value.wrapping_add_signed(addend));
        },
        // S + A - P, with wrapping 64-bit ELF arithmetic truncated to the low 32 bits.
        #[cfg(target_arch = "x86_64")]
        RelocationType::R_X86_64_PC32 => {
            let target: *mut u32 = place as *mut u32;
            let result: u64 = value.wrapping_add_signed(addend).wrapping_sub(place);
            target.write_unaligned(result as u32);
        },
        #[cfg(target_arch = "aarch64")]
        RelocationType::R_AARCH64_JUMP_SLOT
        | RelocationType::R_AARCH64_GLOB_DAT
        | RelocationType::R_AARCH64_ABS64 => {
            let target: *mut u64 = place as *mut u64;
            target.write_unaligned(value.wrapping_add_signed(addend));
        },
        #[cfg(target_arch = "aarch64")]
        RelocationType::R_AARCH64_PREL64 => {
            let target: *mut u64 = place as *mut u64;
            let result: u64 = value.wrapping_add_signed(addend).wrapping_sub(place);
            target.write_unaligned(result);
        },
        _ => return false,
    }

    #[cfg(target_arch = "aarch64")]
    if let Err(error) = ::sys::kcall::mm::__kcall_sync_instruction_cache(
        VirtualAddress::new(place as usize),
        core::mem::size_of::<u64>(),
    ) {
        ::syslog::warn!(
            "dllink_executable(): failed to synchronize instruction cache (place={place:#x}, \
             error={error:?})"
        );
        return false;
    }

    true
}
