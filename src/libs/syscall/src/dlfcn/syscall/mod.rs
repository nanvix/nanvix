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
    vec::Vec,
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

/// Global symbol table for symbols available to dynamically loaded libraries.
///
/// This table is populated from two sources:
/// 1. The main executable's `.dynsym`/`.dynstr` sections (populated via
///    `dlinit()` when the executable is linked with `--export-dynamic`).
/// 2. Shared libraries loaded with `RTLD_GLOBAL` (populated via
///    `register_library_in_global_scope()`).
///
/// When a dynamically loaded library has unresolved symbols, this table
/// is consulted as a last resort, enabling symbol resolution across
/// independently loaded libraries.
static GLOBAL_SYMBOL_TABLE: Lazy<Mutex<BTreeMap<String, usize>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

/// Pinned references to libraries loaded with `RTLD_GLOBAL`.
///
/// When a library is registered in the global scope, an extra `Arc` reference
/// is stored here. This prevents `dlclose()` from actually unloading the
/// library (the `Arc::strong_count` check in `dlclose` will see the extra
/// reference), ensuring that the absolute addresses recorded in
/// `GLOBAL_SYMBOL_TABLE` remain valid for the lifetime of the process.
/// This is equivalent to Linux's `RTLD_NODELETE` behavior for global libraries.
static GLOBAL_PINNED_LIBRARIES: Lazy<Mutex<Vec<Arc<Mutex<DynamicLibrary>>>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

/// Default directories searched when a `DT_NEEDED` entry contains a bare
/// library name (no path separator). Modelled after Linux's default search
/// paths (`/lib`, `/usr/lib`), but simplified to the single `lib/` directory
/// that Nanvix uses for shared libraries.
static LIBRARY_SEARCH_PATHS: Lazy<Mutex<Vec<String>>> =
    Lazy::new(|| Mutex::new(alloc::vec!["lib/".into()]));

/// Ensures `dlinit` runs at most once.
static DLINIT_ONCE: Once = Once::new();

/// Resolves a library filename to a canonical path by searching configured
/// directories.
///
/// All paths are normalized to a consistent form without leading `/`, matching
/// the convention used throughout the Nanvix dlfcn layer and test code (e.g.,
/// `"lib/libmul.so"`). The VFS accepts both relative and absolute paths and
/// normalizes internally, so stripping the leading `/` is safe and ensures
/// that `"/lib/libc.so"`, `"lib/libc.so"`, and the bare DT_NEEDED name
/// `"libc.so"` all resolve to the same canonical path `"lib/libc.so"`.
///
/// If `filename` already contains a path separator (`/`), it is normalized
/// and returned. Otherwise the function tries each directory in
/// [`LIBRARY_SEARCH_PATHS`] in order, returning the first path for which
/// the file exists. If no match is found the bare name is returned so that
/// the subsequent `open_regular_file` call produces the appropriate error.
///
/// NOTE: The probe opens and immediately closes a file descriptor per
/// candidate path. The matched file is re-opened by `DynamicLibrary::open()`.
/// This double-open is accepted for simplicity; a stat-based probe would
/// avoid it but is not currently available in the Nanvix VFS API.
pub(super) fn resolve_library_path(filename: &str) -> String {
    // Strip leading '/' for normalization — the Nanvix dlfcn layer uses
    // relative paths (e.g., "lib/libfoo.so") as the canonical form.
    let filename: &str = filename.trim_start_matches('/');

    // If the filename already contains a path component, use it as-is.
    if filename.contains('/') {
        return String::from(filename);
    }

    // Search configured directories for the bare library name.
    let search_paths = LIBRARY_SEARCH_PATHS.lock();
    for dir in search_paths.iter() {
        let candidate: String = alloc::format!("{}{}", dir, filename);
        // Probe whether the file exists by attempting to open it.
        if let Ok(_fd) = crate::safe::FileSystem::open_regular_file(
            // FileSystemPath::new can fail for invalid names; skip on error.
            &match crate::safe::FileSystemPath::new(&candidate) {
                Ok(p) => p,
                Err(_) => continue,
            },
            &crate::safe::RegularFileOpenFlags::read_only(),
            None,
        ) {
            ::syslog::debug!("resolve_library_path(): resolved '{}' -> '{}'", filename, candidate);
            return candidate;
        }
    }

    // Fall back to the original name (will produce a clear error at open time).
    ::syslog::debug!("resolve_library_path(): no match for '{}', using as-is", filename);
    String::from(filename)
}

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

/// Registers all defined (non-undefined) symbols from a loaded shared library
/// into the global symbol table. Called when a library is loaded with
/// `RTLD_GLOBAL` so its symbols are available for subsequently loaded libraries.
///
/// The library is also pinned via an extra `Arc` reference in
/// `GLOBAL_PINNED_LIBRARIES`, preventing `dlclose()` from unloading it and
/// leaving dangling addresses in the global symbol table.
///
/// Repeated calls for the same library are idempotent — duplicate pins are
/// not created and already-registered symbols are not overwritten.
///
/// NOTE: Per Linux semantics, symbols registered here are NOT removed when the
/// library is closed via `dlclose()`. This matches glibc behavior where global
/// scope entries persist even after the originating library is unloaded.
pub(super) fn register_library_in_global_scope(
    lib_arc: &Arc<Mutex<DynamicLibrary>>,
) {
    // Check if this library is already pinned (idempotent promotion).
    {
        let pinned: spin::MutexGuard<'_, Vec<Arc<Mutex<DynamicLibrary>>>> =
            GLOBAL_PINNED_LIBRARIES.lock();
        if pinned.iter().any(|p| Arc::ptr_eq(p, lib_arc)) {
            return;
        }
    }

    let lib: spin::MutexGuard<'_, DynamicLibrary> = lib_arc.lock();
    let mut table = GLOBAL_SYMBOL_TABLE.lock();
    let mut count: usize = 0;

    for (name, addr) in lib.exported_symbols() {
        use ::alloc::collections::btree_map::Entry;
        if let Entry::Vacant(e) = table.entry(String::from(name)) {
            e.insert(addr);
            count += 1;
        }
    }

    ::syslog::trace!(
        "register_library_in_global_scope(): registered {} symbols from {:?}",
        count,
        lib.name()
    );

    // Drop locks before pinning to avoid holding both at once.
    drop(table);
    drop(lib);

    // Pin the library so dlclose() cannot unload it while its symbols
    // are recorded in the global table.
    GLOBAL_PINNED_LIBRARIES.lock().push(lib_arc.clone());
}

//==================================================================================================
// Exports
//==================================================================================================

pub use dladdr::dladdr;
pub use dlclose::dlclose;
pub use dlopen::dlopen;
pub use dlsym::dlsym;
