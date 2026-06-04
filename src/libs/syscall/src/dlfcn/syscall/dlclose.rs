// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use super::dynlib::DlHandle;
use crate::dlfcn::syscall::{
    dynlib::DynamicLibrary,
    DYNAMIC_LIBRARY_REGISTRY,
};
use ::alloc::{
    collections::btree_map::BTreeMap,
    string::String,
    sync::Arc,
    vec::Vec,
};
use ::spin::{
    Mutex,
    MutexGuard,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// dclose()
//==================================================================================================

/// Closes a dynamic library file.
pub fn dlclose(handle: &DlHandle) -> Result<(), Error> {
    ::syslog::trace!("dlclose(): handle={:?}", handle);

    let mut registry: MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>> =
        DYNAMIC_LIBRARY_REGISTRY.lock();

    // Check if dynamic library file is opened.
    if !registry.contains_key(handle) {
        let reason: &str = "dynamic library file not open";
        ::syslog::warn!("dlclose(): {}", reason);
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    let mut dep_dlfiles: Vec<Arc<Mutex<DynamicLibrary>>> = {
        let mut dlfile: Vec<(DlHandle, Arc<Mutex<DynamicLibrary>>)> = registry
        // Check if dynamic library should be removed from registry.
            .extract_if(.., |dlhandle, dlfile| {
                // Check if the handle matches the dynamic library name.
                if handle == dlhandle {
                    // Check if this is the only remaining reference to the dynamic library file.
                    Arc::strong_count(dlfile) == 1
                } else {
                    false
                }
            })
            .collect();

        // Dynamic library file is still in use.
        if dlfile.is_empty() {
            return Ok(());
        }

        assert_eq!(
            dlfile.len(),
            1,
            "dlclose(): expected to remove exactly one dynamic library file"
        );

        // Collect all dependencies of the dynamic library file being closed.
        let mut dep_dlfiles: Vec<Arc<Mutex<DynamicLibrary>>> = Vec::new();
        if let Some((_, dep_dlfile)) = dlfile.pop() {
            // Snapshot the `.fini_array` descriptor and name under a short
            // per-library lock, then drop the lock and invoke destructors.
            // Holding the per-library lock during destructor execution
            // would deadlock any destructor that calls
            // `dlsym(self_handle, ...)`. (The outer registry lock is still
            // held, so destructors that call `dlopen` / `dlclose` / cross-
            // library `dlsym` are not yet supported; see the loader docs.)
            let (descriptor, name): (Option<(usize, usize)>, String) = {
                let lib = dep_dlfile.lock();
                (lib.fini_array_descriptor(), String::from(lib.name()))
            };
            // SAFETY: `descriptor` was produced under the per-library lock
            // for a library still alive via `dep_dlfile`; the per-library
            // mutex is released before invocation so destructors that call
            // `dlsym(self_handle, ...)` do not deadlock.
            unsafe {
                DynamicLibrary::invoke_fini_array(descriptor, &name);
            }

            let mut dep_dlfile = dep_dlfile.lock();
            dep_dlfile
                .take_dependencies()
                .iter()
                .for_each(|(_dlname, dlfile)| {
                    dep_dlfiles.push(dlfile.clone());
                });
        }

        dep_dlfiles
    };

    while let Some(dep_dlfile) = dep_dlfiles.pop() {
        // Check if dynamic library should be removed from registry.
        let mut dep_dlfile: Vec<(DlHandle, Arc<Mutex<DynamicLibrary>>)> = registry
            .extract_if(.., |dlhandle, dlfile| {
                // Check if the handle matches the dynamic library.
                if &dep_dlfile.lock().handle() == dlhandle {
                    // Check if this is the only remaining reference to the dynamic library file.
                    // One reference in the registry and one hold by this loop.
                    Arc::strong_count(dlfile) == 2
                } else {
                    false
                }
            })
            .collect();

        assert_eq!(
            dep_dlfile.len(),
            1,
            "dlclose(): expected to remove exactly one dynamic library file"
        );

        // Collect all dependencies of the dynamic library file.
        if let Some((_, dep_dlfile)) = dep_dlfile.pop() {
            // Run `.fini_array` destructors before this dependency is
            // dropped. See the comment above on lock handling.
            let (descriptor, name): (Option<(usize, usize)>, String) = {
                let lib = dep_dlfile.lock();
                (lib.fini_array_descriptor(), String::from(lib.name()))
            };
            // SAFETY: see the comment on the first `invoke_fini_array`
            // call above.
            unsafe {
                DynamicLibrary::invoke_fini_array(descriptor, &name);
            }

            let mut dep_dlfile = dep_dlfile.lock();
            dep_dlfile
                .take_dependencies()
                .iter()
                .for_each(|(_dlname, dlfile)| {
                    dep_dlfiles.push(dlfile.clone());
                });
            ::syslog::debug!(
                "dlclose(): closing dependency (name={:?}, registry.len={:?})",
                dep_dlfile.name(),
                registry.len()
            );
        }
    }

    Ok(())
}
