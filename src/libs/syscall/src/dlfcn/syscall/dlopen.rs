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
pub fn dlopen(filename: &str) -> Result<DlHandle, Error> {
    ::syslog::trace!("dlopen(): filename={}", filename);

    // Ensure the global symbol table is populated so that symbols exported
    // by the main executable can be resolved during relocation, even if the
    // caller never invoked dlopen(NULL). Guarded by Once, so subsequent
    // calls are a no-op.
    super::dlinit();

    // TODO: Normalize filename.

    let mut registry: MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>> =
        DYNAMIC_LIBRARY_REGISTRY.lock();

    // Check if dynamic library is already opened.
    for (dlhandle, dlfile) in registry.iter() {
        if dlfile.lock().name() == filename {
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
        Ok(()) => Ok(handle),
        Err(e) => {
            let new_handles: Vec<DlHandle> = registry
                .keys()
                .filter(|h| !handles_before.contains(h))
                .copied()
                .collect();
            ::syslog::error!(
                "dlopen(): rolling back {} entries after failure (error={:?})",
                new_handles.len(),
                e
            );
            for h in new_handles {
                registry.remove(&h);
            }
            Err(e)
        },
    }
}

/// Recursively loads all transitive dependencies of a newly opened library.
///
/// For each `DT_NEEDED` entry, checks if it is already loaded in the registry,
/// and if not, opens and inserts it. Recurses into each dependency's own
/// `DT_NEEDED` entries.
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
            for (dlhandle, dlfile) in dlfiles.iter() {
                // Check if need to skip the dynamic library itself.
                if dlhandle == new_dlhandle {
                    continue;
                }

                if dlfile.lock().name() == dependency.as_str() {
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
            // Open and pre-load the dynamic library file.
            let dep_dlfile: DynamicLibrary = DynamicLibrary::open(&dependency)?;
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
/// Every library whose handle is NOT in `handles_before` is a new entry from
/// the current `dlopen` invocation and needs its relocations patched.
fn resolve_all_symbols(
    dlfiles: &mut MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>>,
    handles_before: &BTreeSet<DlHandle>,
) -> Result<(), Error> {
    ::syslog::trace!("resolve_all_symbols()");

    // Resolve relocations for every library added during this dlopen call,
    // not just the root. This ensures transitive dependencies loaded via
    // DT_NEEDED also have their relocations patched.
    let new_handles: Vec<DlHandle> = dlfiles
        .keys()
        .filter(|h| !handles_before.contains(h))
        .copied()
        .collect();

    for handle in new_handles {
        if let Some(dlfile) = dlfiles.get(&handle) {
            dlfile.lock().resolve_all()?;
        }
    }

    Ok(())
}
