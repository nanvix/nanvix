// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use super::dynlib::{
    DlHandle,
    DynamicLibrary,
};
use crate::dlfcn::syscall::DYNAMIC_LIBRARY_REGISTRY;
use ::alloc::{
    collections::{
        btree_map::BTreeMap,
        btree_set::BTreeSet,
    },
    string::{
        String,
        ToString,
    },
    sync::Arc,
    vec::Vec,
};
use ::spin::{
    Mutex,
    MutexGuard,
};
use ::sys::error::Error;

//===================================================================================================
// dlopen()
//==================================================================================================

/// Opens a dynamic library file.
pub fn dlopen(filename: &str, global: bool) -> Result<DlHandle, Error> {
    ::syslog::trace!("dlopen(): filename={}, global={}", filename, global);

    // Ensure the global symbol table is populated so that symbols exported
    // by the main executable can be resolved during relocation, even if the
    // caller never invoked dlopen(NULL). Guarded by Once, so subsequent
    // calls are a no-op.
    super::dlinit();

    // Resolve bare library names (e.g., "libfoo.so") to a canonical path
    // (e.g., "lib/libfoo.so") using the configured search directories.
    // Paths with leading "/" are normalized to strip it, ensuring
    // "/lib/libc.so" and "lib/libc.so" are treated as the same library.
    let filename: &str = &super::resolve_library_path(filename);

    let mut registry: MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>> =
        DYNAMIC_LIBRARY_REGISTRY.lock();

    // Check if dynamic library is already opened. The filename has been
    // resolved by resolve_library_path() above, so this comparison works
    // correctly even for DT_NEEDED bare names that were resolved to full
    // paths on initial load.
    for (dlhandle, dlfile) in registry.iter() {
        if dlfile.lock().name() == filename {
            // If the caller requests RTLD_GLOBAL on a library that was
            // previously loaded without it, promote it to global scope now.
            if global {
                super::register_library_in_global_scope(dlfile);
            }
            return Ok(*dlhandle);
        }
    }

    // Snapshot the registry keys before we start inserting, so we can roll back
    // all new entries on failure. This ensures a failed dlopen never leaves
    // stale handles with unpatched relocations in the registry.
    let handles_before: BTreeSet<DlHandle> = registry.keys().copied().collect();

    // Open and pre-load the dynamic library file.
    let new_dlfile: DynamicLibrary = DynamicLibrary::open(filename)?;
    let handle: DlHandle = new_dlfile.handle();
    let new_dlfile: Arc<Mutex<DynamicLibrary>> = Arc::new(Mutex::new(new_dlfile));

    // Insert the opened file into the map.
    registry.insert(handle, new_dlfile.clone());

    // Load dependencies and resolve symbols. If either step fails, remove all
    // entries that were added during this call (the library itself and any
    // transitive dependencies) so subsequent dlopen calls start fresh.
    match load_all_dependencies(&mut registry, new_dlfile)
        .and_then(|_| resolve_all_symbols(&mut registry, &handles_before))
    {
        Ok(()) => {
            // If RTLD_GLOBAL was requested, publish the library's exported
            // symbols into the global symbol table so subsequently loaded
            // libraries can resolve them.
            if global {
                if let Some(dlfile) = registry.get(&handle) {
                    super::register_library_in_global_scope(dlfile);
                }
            }
            Ok(handle)
        },
        Err(e) => {
            let new_handles: Vec<DlHandle> = registry
                .keys()
                .filter(|h| !handles_before.contains(h))
                .copied()
                .collect();
            for h in new_handles {
                registry.remove(&h);
            }
            Err(e)
        },
    }
}

/// Recursively loads all transitive dependencies of a newly opened library.
///
/// For each `DT_NEEDED` entry, resolves the bare library name to a full path,
/// checks if it is already loaded in the registry, and if not, opens and inserts
/// it. Recurses into each dependency's own `DT_NEEDED` entries.
fn load_all_dependencies(
    dlfiles: &mut MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>>,
    new_dlfile: Arc<Mutex<DynamicLibrary>>,
) -> Result<(), Error> {
    ::syslog::trace!("load_all_dependencies(): new_dlfile={:?}", new_dlfile.lock().name());

    fn load_all_dependencies_recursive(
        dlfiles: &mut MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>>,
        new_dlhandle: &DlHandle,
        new_dlfile: &mut MutexGuard<'_, DynamicLibrary>,
    ) -> Result<(), Error> {
        // Collect the name of all dependencies.
        let mut dependencies: Vec<String> = new_dlfile
            .dependencies()
            .keys()
            .map(|dlname| dlname.to_string())
            .collect();

        // Bind to already loaded dependencies and remove them from the list.
        dependencies.retain(|dependency| {
            // Resolve bare name so we can match against loaded libraries
            // that were opened with a full path.
            let resolved_dep: String = super::resolve_library_path(dependency);
            for (dlhandle, dlfile) in dlfiles.iter() {
                // Check if need to skip the dynamic library itself.
                if dlhandle == new_dlhandle {
                    continue;
                }

                let loaded_name = dlfile.lock().name().to_string();
                if loaded_name == dependency.as_str() || loaded_name == resolved_dep {
                    ::syslog::debug!(
                        "load_all_dependencies_recursive(): already loaded dependency '{}' \
                         (handle={:?})",
                        dependency,
                        dlhandle
                    );
                    // Update the dependency to the already loaded file.
                    if let Err(_error) =
                        new_dlfile.bind_dependency(dependency.clone(), dlfile.clone())
                    {
                        // TODO: comment
                        unreachable!("cannot fail to bind dependency");
                    }

                    return false;
                }
            }
            true
        });

        // Load remaining dependencies.
        while let Some(dependency) = dependencies.pop() {
            // Resolve bare library names to full paths using search directories.
            let resolved_dep: String = super::resolve_library_path(&dependency);

            // Open and pre-load the dynamic library file.
            let dep_dlfile: DynamicLibrary = DynamicLibrary::open(&resolved_dep)?;
            let handle: DlHandle = dep_dlfile.handle();
            let dep_dlfile: Arc<Mutex<DynamicLibrary>> = Arc::new(Mutex::new(dep_dlfile));

            // Insert the opened file into the map.
            if let Some(dlfile) = dlfiles.insert(handle, dep_dlfile.clone()) {
                unreachable!("dlopen(): library file already loaded (dlfile={:?})", dlfile);
            }

            new_dlfile.bind_dependency(dependency.clone(), dep_dlfile.clone())?;

            // Load dependencies of the new dynamic library file.
            let mut dlfile: MutexGuard<'_, DynamicLibrary> = dep_dlfile.lock();
            load_all_dependencies_recursive(dlfiles, &handle, &mut dlfile)?;
        }

        Ok(())
    }

    let mut new_dlfile = new_dlfile.lock();
    let new_dlhandle = new_dlfile.handle();
    load_all_dependencies_recursive(dlfiles, &new_dlhandle, &mut new_dlfile)?;

    Ok(())
}

/// Resolves all relocations for libraries added during this `dlopen` call.
///
/// Libraries are resolved in dependency order (leaves first, root last).
/// This ensures that when a library's relocations reference symbols from
/// its dependencies, those dependencies are already fully resolved.
fn resolve_all_symbols(
    dlfiles: &mut MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>>,
    handles_before: &BTreeSet<DlHandle>,
) -> Result<(), Error> {
    ::syslog::trace!("resolve_all_symbols()");

    // Collect all newly added libraries.
    let new_handles: Vec<DlHandle> = dlfiles
        .keys()
        .filter(|h| !handles_before.contains(h))
        .copied()
        .collect();

    // Build a resolution order: resolve dependencies before their dependents.
    // A library can be resolved once all its bound dependencies (that are also
    // new in this dlopen call) have been resolved.
    let mut resolved: BTreeSet<DlHandle> = BTreeSet::new();
    let mut ordered: Vec<DlHandle> = Vec::with_capacity(new_handles.len());

    // Iteratively find libraries whose dependencies are all resolved.
    // This is a simple topological sort — safe because the dependency graph
    // is a DAG (cycles would have been caught during load_all_dependencies).
    loop {
        let mut progress: bool = false;
        for &handle in &new_handles {
            if resolved.contains(&handle) {
                continue;
            }

            // Check if all of this library's dependencies that are new in
            // this dlopen call have been resolved.
            let all_deps_resolved: bool = if let Some(dlfile) = dlfiles.get(&handle) {
                let lib: spin::MutexGuard<'_, DynamicLibrary> = dlfile.lock();
                lib.dependency_handles().iter().all(|dep_handle| {
                    // Dependencies that existed before this dlopen are already resolved.
                    handles_before.contains(dep_handle) || resolved.contains(dep_handle)
                })
            } else {
                true
            };

            if all_deps_resolved {
                ordered.push(handle);
                resolved.insert(handle);
                progress = true;
            }
        }

        if resolved.len() == new_handles.len() {
            break;
        }

        if !progress {
            // No progress means a cycle or missing dependency. Fall back to
            // resolving remaining libraries in arbitrary order.
            ::syslog::warn!(
                "resolve_all_symbols(): could not determine dependency order for {} libraries",
                new_handles.len() - resolved.len()
            );
            for &handle in &new_handles {
                if !resolved.contains(&handle) {
                    ordered.push(handle);
                }
            }
            break;
        }
    }

    // Resolve in dependency order (leaves first).
    for handle in ordered {
        if let Some(dlfile) = dlfiles.get(&handle) {
            dlfile.lock().resolve_all()?;
        }
    }

    Ok(())
}
