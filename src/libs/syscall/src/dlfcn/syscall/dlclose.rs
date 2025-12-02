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
        ::syslog::error!("dlclose(): {}", reason);
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
