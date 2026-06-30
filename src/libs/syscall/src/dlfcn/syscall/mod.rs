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
mod exe_link;

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

/// Ensures the global-scope `.fini_array` destructors run at most once.
static GLOBAL_FINI_ONCE: Once = Once::new();

/// Resolves a library filename to a canonical path.
///
/// If `filename` contains a path separator (`/`), it is treated as an explicit
/// path (absolute or relative with directory component).  Only a leading
/// `./` is stripped (`./foo` is equivalent to `foo` for `openat()`
/// purposes); the leading `/` of an absolute path is preserved as-is.
/// This matches Linux behavior, where absolute and relative paths bypass
/// `LD_LIBRARY_PATH` and are passed straight through to `open()`.
///
/// If `filename` is a bare name (no `/`), the function tries each directory in
/// the following order, returning the first path for which the file exists:
///
/// 1. The directories listed in `runpaths` (caller-supplied, typically the
///    `DT_RUNPATH` entries of the library whose dependency is being resolved).
///    `$ORIGIN` inside an entry is substituted with `"."` (the current
///    working directory). This is a temporary approximation; the System V
///    `ld.so` convention is to substitute the directory of the loading
///    library, which Nanvix does not yet track per-library.
/// 2. The directories in [`LIBRARY_SEARCH_PATHS`] (currently just `"lib/"`).
///
/// If no match is found the bare name is returned so that the subsequent
/// `open_regular_file` call produces the appropriate error.
///
/// # Absolute paths must stay absolute
///
/// Earlier revisions of this function also called `.trim_start_matches('/')`
/// on absolute paths, on the assumption that the VFS accepted both forms
/// and that this produced a more canonical form for the
/// already-loaded-library lookup in `dlopen`.  In practice that conversion
/// makes the subsequent `openat()` resolve the path against the caller's
/// CWD instead of the filesystem root, which silently breaks
/// `dlopen("/lib/foo.so")` for any caller whose CWD is not `/`.
/// CPython's `regrtest` (which sets `TMPDIR=/tmp` and `chdir`s before
/// running tests) is the canonical reproducer.
///
/// NOTE: The probe opens and immediately closes a file descriptor per
/// candidate path. The matched file is re-opened by `DynamicLibrary::open()`.
/// This double-open is accepted for simplicity; a stat-based probe would
/// avoid it but is not currently available in the Nanvix VFS API.
///
/// `DT_RPATH` (the predecessor to `DT_RUNPATH`) is intentionally not consulted:
/// it is deprecated by the System V gABI and modern toolchains emit
/// `DT_RUNPATH` instead.
pub(super) fn resolve_library_path(filename: &str, runpaths: Option<&[String]>) -> String {
    // If the original filename contains a path separator, the caller provided
    // an explicit path (absolute or relative with directory). Normalize it
    // but do NOT search configured directories — matching Linux behavior
    // where absolute/relative paths bypass `LD_LIBRARY_PATH`.
    if filename.contains('/') {
        // `./foo` is equivalent to `foo` for `openat()` purposes, so strip
        // a leading `./` if present.  Do NOT strip a leading `/` — that
        // would convert an absolute path into a relative one and break
        // resolution when CWD != `/` (see doc comment above).
        let normalized: &str = match filename.strip_prefix("./") {
            Some(rest) => rest.trim_start_matches('/'),
            None => filename,
        };
        // Guard against pathological input like `./` becoming empty.
        if normalized.is_empty() {
            return String::from(filename);
        }
        return String::from(normalized);
    }

    // Bare library name (no path separator) — search runpaths first, then
    // the configured default directories.
    if let Some(runpaths) = runpaths {
        for dir in runpaths.iter() {
            let dir: String = substitute_origin(dir);
            let candidate: String = join_dir(&dir, filename);
            if probe_exists(&candidate) {
                // Apply the same canonicalization as the explicit-path
                // branch above so callers that compare against already-
                // loaded library names (which are stored canonically)
                // don't see duplicates like `"./libfoo.so"` vs
                // `"libfoo.so"`. A single leading "./" is stripped, but a
                // leading "/" is preserved so that an absolute DT_RUNPATH
                // entry (e.g. `"/usr/lib"`) keeps resolving against the
                // filesystem root rather than the caller's CWD.
                let canonical: String = canonicalize(&candidate);
                ::syslog::debug!(
                    "resolve_library_path(): resolved '{}' via DT_RUNPATH -> '{}'",
                    filename,
                    canonical
                );
                return canonical;
            }
        }
    }

    // Clone the path list under the lock, then release it before probing
    // the filesystem (which involves I/O and should not hold a spinlock).
    let search_paths: Vec<String> = LIBRARY_SEARCH_PATHS.lock().clone();
    for dir in search_paths.iter() {
        let candidate: String = join_dir(dir, filename);
        if probe_exists(&candidate) {
            let canonical: String = canonicalize(&candidate);
            ::syslog::debug!("resolve_library_path(): resolved '{}' -> '{}'", filename, canonical);
            return canonical;
        }
    }

    // Fall back to the original name (will produce a clear error at open time).
    ::syslog::debug!("resolve_library_path(): no match for '{}', using as-is", filename);
    String::from(filename)
}

/// Normalizes a resolved candidate path to the canonical form used throughout
/// the dlfcn layer (e.g. `"lib/libc.so"`), matching the explicit-path branch of
/// [`resolve_library_path`]: a single leading `./` is stripped, but a leading
/// `/` is preserved so that absolute paths (such as those produced from an
/// absolute `DT_RUNPATH` entry) stay absolute and resolve against the
/// filesystem root regardless of the caller's CWD. Returns the input unchanged
/// if normalization would yield an empty string.
fn canonicalize(path: &str) -> String {
    let stripped: &str = match path.strip_prefix("./") {
        Some(rest) => rest.trim_start_matches('/'),
        None => path,
    };
    if stripped.is_empty() {
        String::from(path)
    } else {
        String::from(stripped)
    }
}

/// Joins a directory and filename without introducing duplicate separators.
fn join_dir(dir: &str, filename: &str) -> String {
    if dir.is_empty() {
        return String::from(filename);
    }
    if dir.ends_with('/') {
        alloc::format!("{}{}", dir, filename)
    } else {
        alloc::format!("{}/{}", dir, filename)
    }
}

/// Substitutes `$ORIGIN` in a runpath entry. Nanvix has no per-process loader
/// directory concept yet, so `$ORIGIN` is currently expanded to `"."` (the
/// current working directory). This is a temporary approximation: the System V
/// `ld.so` convention is to expand `$ORIGIN` to the directory containing the
/// loading library, which Nanvix does not yet track per-library. Keeping a
/// well-formed substitution (rather than the empty string) avoids producing
/// invalid paths like `"//lib"` from entries such as `"$ORIGIN/lib"`.
fn substitute_origin(entry: &str) -> String {
    if !entry.contains("$ORIGIN") {
        return String::from(entry);
    }
    entry.replace("$ORIGIN", ".")
}

/// Returns `true` if a regular file exists at `candidate`.
fn probe_exists(candidate: &str) -> bool {
    let path: crate::safe::FileSystemPath = match crate::safe::FileSystemPath::new(candidate) {
        Ok(p) => p,
        Err(_) => return false,
    };
    crate::safe::FileSystem::open_regular_file(
        &path,
        &crate::safe::RegularFileOpenFlags::read_only(),
        None,
    )
    .is_ok()
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

/// Looks up a symbol in the global symbol table.
///
/// Searches symbols from the main executable and any libraries registered
/// with `RTLD_GLOBAL`. Ensures the table is populated before the first lookup.
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
/// NOTE: Symbols registered here are NOT removed when the library is closed
/// via `dlclose()`. The library is pinned (via `GLOBAL_PINNED_LIBRARIES`) to
/// prevent actual unloading, ensuring the recorded absolute addresses remain
/// valid for the lifetime of the process.
pub(super) fn register_library_in_global_scope(lib_arc: &Arc<Mutex<DynamicLibrary>>) {
    // Hold the pin lock across the entire check-and-insert to prevent
    // duplicate pins from concurrent calls (TOCTOU).
    let mut pinned: spin::MutexGuard<'_, Vec<Arc<Mutex<DynamicLibrary>>>> =
        GLOBAL_PINNED_LIBRARIES.lock();
    if pinned.iter().any(|p| Arc::ptr_eq(p, lib_arc)) {
        return;
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

    drop(table);
    drop(lib);

    // Pin the library so dlclose() cannot unload it while its symbols
    // are recorded in the global table.
    pinned.push(lib_arc.clone());
}

/// Runs the `.fini_array` destructors of every shared library that was loaded
/// into the global scope (`RTLD_GLOBAL`) in the reverse of their load order.
///
/// This is the process-exit counterpart of the `.init_array` constructors that
/// [`dlopen`] runs when a library is loaded. The startup `DT_NEEDED` loader
/// ([`dllink_executable`](exe_link::dllink_executable)) loads the executable's
/// dependencies with `RTLD_GLOBAL`, so each one is pinned in
/// [`GLOBAL_PINNED_LIBRARIES`] in load order and its constructors run before
/// `main`. Those pinned libraries are never unloaded by `dlclose` (the extra
/// pin reference keeps their reference count above the unload threshold), so
/// their destructors would otherwise never run. This routine closes that gap
/// by invoking them at process teardown, newest-loaded first — the reverse of
/// construction order, as required by the System V gABI.
///
/// Idempotent: a [`Once`] guard ensures the destructors run at most once even
/// if this is called more than once (e.g. from multiple exit paths).
///
/// # Locking
///
/// The pinned-library list is snapshotted under its lock and the lock is
/// released before any destructor runs, so a destructor may legally re-enter
/// the loader (`dlsym`/`dlopen`/`dlclose`) without self-deadlocking. The
/// snapshot's `Arc`s keep every library — and therefore its mapped segments —
/// alive for the duration of the walk.
pub(super) fn run_global_destructors() {
    GLOBAL_FINI_ONCE.call_once(|| {
        // Snapshot the pinned libraries in reverse load order, then release the
        // lock before invoking any destructor.
        let libraries: Vec<Arc<Mutex<DynamicLibrary>>> = {
            let pinned: spin::MutexGuard<'_, Vec<Arc<Mutex<DynamicLibrary>>>> =
                GLOBAL_PINNED_LIBRARIES.lock();
            pinned.iter().rev().cloned().collect()
        };

        for lib_arc in libraries.iter() {
            // Snapshot the destructor descriptor and library name under a short
            // per-library lock, then drop the lock before invoking destructors
            // so a destructor that re-enters the loader cannot deadlock.
            let (descriptor, name): (Option<(usize, usize)>, String) = {
                let lib: spin::MutexGuard<'_, DynamicLibrary> = lib_arc.lock();
                (lib.fini_array_descriptor(), String::from(lib.name()))
            };
            // SAFETY: `descriptor` was produced under the per-library lock for a
            // library still held alive by `libraries`'s `Arc`; pinned libraries
            // are never unloaded, so its segments remain mapped; and no dlfcn
            // locks are held across this call.
            unsafe {
                DynamicLibrary::invoke_fini_array(descriptor, &name);
            }
        }
    });
}

//==================================================================================================
// Exports
//==================================================================================================

pub use dladdr::dladdr;
pub use dlclose::dlclose;
pub use dlopen::dlopen;
pub use dlsym::dlsym;
pub use exe_link::{
    dlfini_executable,
    dllink_executable,
};
