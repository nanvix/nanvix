// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//====================================================================================================

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]

//==================================================================================================
// Modules
//==================================================================================================

mod dladdr;
mod dlclose;
mod dlopen;
mod dlsym;
mod dynlib;

//==================================================================================================
// Imports
//===================================================================================================

pub use self::dynlib::DlHandle;

use self::dynlib::DynamicLibrary;
use ::alloc::{
    collections::btree_map::BTreeMap,
    string::String,
    sync::Arc,
};
use ::elf::{
    StringTable,
    SymbolTable,
};
use ::spin::{
    Lazy,
    Mutex,
    Once,
};

//==================================================================================================

static DYNAMIC_LIBRARY_REGISTRY: Lazy<Mutex<BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

/// Global symbol table for symbols exported by the main executable.
/// When a dynamically loaded library has unresolved symbols, this table
/// is consulted as a last resort, enabling statically-linked executables
/// to export symbols to their dynamically-loaded extension modules.
///
/// NOTE: this table only contains symbols from the main executable
/// (populated from `.dynsym`/`.dynstr` sections emitted by
/// `--export-dynamic`). Symbols from dynamically loaded shared
/// libraries are *not* included; those are resolved through
/// the per-library dependency chains in `DYNAMIC_LIBRARY_REGISTRY`.
static GLOBAL_SYMBOL_TABLE: Lazy<Mutex<BTreeMap<String, usize>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

/// Ensures `dlinit` runs at most once.
static DLINIT_ONCE: Once = Once::new();

/// Populates `GLOBAL_SYMBOL_TABLE` from the executable's `.dynsym`/`.dynstr`
/// sections. The linker script emits `__dynsym_start/__dynsym_end` and
/// `__dynstr_start/__dynstr_end` boundary symbols around these sections when
/// the executable is linked with `--export-dynamic`.
///
/// If the executable was linked without `--export-dynamic` (boundaries are
/// equal), this function is a harmless no-op.
pub fn dlinit() {
    DLINIT_ONCE.call_once(|| {
        // SAFETY: These symbols are defined by the linker script and point to
        // valid in-memory sections that are part of the loaded executable image.
        let (dynsym_start, dynsym_end, dynstr_start, dynstr_end) = unsafe {
            extern "C" {
                static __dynsym_start: u8;
                static __dynsym_end: u8;
                static __dynstr_start: u8;
                static __dynstr_end: u8;
            }
            (
                &__dynsym_start as *const u8 as usize,
                &__dynsym_end as *const u8 as usize,
                &__dynstr_start as *const u8 as usize,
                &__dynstr_end as *const u8 as usize,
            )
        };

        let dynsym_size: usize = dynsym_end.saturating_sub(dynsym_start);
        let dynstr_size: usize = dynstr_end.saturating_sub(dynstr_start);

        // Nothing to do if the executable has no dynamic symbol table.
        if dynsym_size == 0 || dynstr_size == 0 {
            ::syslog::trace!("dlinit(): no .dynsym/.dynstr sections found");
            return;
        }

        let sym_entry_size: usize = core::mem::size_of::<elf::Symbol>();
        let sym_count: usize = dynsym_size / sym_entry_size;

        // SAFETY: The linker-script boundaries guarantee that these pointers
        // span valid, correctly-aligned ELF symbol and string table data that
        // is part of the loaded executable image and will not be deallocated.
        let dynsym =
            unsafe { SymbolTable::from_raw_parts(dynsym_start as *mut elf::Symbol, sym_count) };
        let dynstr = unsafe { StringTable::from_raw_parts(dynstr_start as *const u8, dynstr_size) };

        let mut table = GLOBAL_SYMBOL_TABLE.lock();
        let mut count: usize = 0;

        for sym in dynsym.iter() {
            // Skip undefined symbols; names are further validated below.
            if sym.is_undefined() {
                continue;
            }
            if let Ok(name) = dynstr.get_name(sym.name_offset()) {
                if !name.is_empty() {
                    use ::alloc::collections::btree_map::Entry;
                    if let Entry::Vacant(e) = table.entry(String::from(name)) {
                        e.insert(sym.value() as usize);
                        count += 1;
                    }
                }
            }
        }

        ::syslog::trace!("dlinit(): registered {} symbols from executable", count);
    });
}

/// Looks up a symbol in the global symbol table (main executable only).
///
/// Ensures the table is populated before the first lookup.
pub(super) fn global_symbol_lookup(name: &str) -> Option<usize> {
    dlinit();
    GLOBAL_SYMBOL_TABLE.lock().get(name).copied()
}

//==================================================================================================
// Exports
//==================================================================================================

pub use dladdr::dladdr;
pub use dlclose::dlclose;
pub use dlopen::dlopen;
pub use dlsym::dlsym;
