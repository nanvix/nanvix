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
    collections::btree_map::BTreeMap,
    string::{
        String,
        ToString,
    },
    sync::Arc,
    vec,
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

    // Open and pre-load the dynamic library file.
    let new_dlfile: DynamicLibrary = DynamicLibrary::open(filename)?;
    let handle: DlHandle = new_dlfile.handle();
    let new_dlfile: Arc<Mutex<DynamicLibrary>> = Arc::new(Mutex::new(new_dlfile));

    // Insert the opened file into the map.
    registry.insert(handle, new_dlfile.clone());

    load_all_dependencies(&mut registry, new_dlfile)?;
    resolve_all_symbols(&mut registry, filename)?;

    Ok(handle)
}

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

fn resolve_all_symbols(
    dlfiles: &mut MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>>,
    filename: &str,
) -> Result<(), Error> {
    ::syslog::trace!("resolve_all_symbols(): filename={}", filename);
    let mut unresolved_libraries = vec![filename.to_string()];

    while let Some(lib_name) = unresolved_libraries.pop() {
        if let Some(dlfile) = dlfiles.values().find(|f| f.lock().name() == lib_name) {
            dlfile.lock().resolve_all()?;
        }
    }

    Ok(())
}
