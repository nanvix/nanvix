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

    // Order in which `.fini_array` destructors must run: the closing library
    // first, then its dependencies (the reverse of constructor order). The
    // libraries are removed from the registry while `DYNAMIC_LIBRARY_REGISTRY`
    // is held, but their destructors run *after* the lock is released so a
    // destructor may legally call back into `dlsym`/`dlopen`/`dlclose` without
    // self-deadlocking on the registry mutex.
    let fini_order: Vec<Arc<Mutex<DynamicLibrary>>> = {
        let mut registry: MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>> =
            DYNAMIC_LIBRARY_REGISTRY.lock();

        // Check if dynamic library file is opened.
        if !registry.contains_key(handle) {
            let reason: &str = "dynamic library file not open";
            ::syslog::warn!("dlclose(): {}", reason);
            return Err(Error::new(ErrorCode::BadFile, reason));
        }

        // Remove the closing library from the registry, but only if this is the
        // last reference to it (i.e., no other loaded library still depends on
        // it). If it is still referenced, there is nothing to unload yet.
        let mut extracted: Vec<(DlHandle, Arc<Mutex<DynamicLibrary>>)> = registry
            .extract_if(.., |dlhandle, dlfile| handle == dlhandle && Arc::strong_count(dlfile) == 1)
            .collect();

        // A handle is unique, so at most one entry can match.
        debug_assert!(extracted.len() <= 1);

        // Dynamic library file is still in use.
        let root: Arc<Mutex<DynamicLibrary>> = match extracted.pop() {
            Some((_, root)) => root,
            None => return Ok(()),
        };

        // Walk the dependency graph, removing every library that becomes
        // unreferenced once its dependents are removed, recording the unload
        // order (dependents before dependencies). `take_dependencies` detaches
        // a library's dependency edges, so recording a library here drops its
        // hold on its dependencies; a dependency is only unloaded once *all* of
        // its dependents -- inside and outside this `dlclose` call -- have
        // released it.
        let mut fini_order: Vec<Arc<Mutex<DynamicLibrary>>> = Vec::new();
        let mut worklist: Vec<Arc<Mutex<DynamicLibrary>>> = Vec::new();

        for (_dlname, dep) in root.lock().take_dependencies() {
            worklist.push(dep);
        }
        fini_order.push(root);

        while let Some(candidate) = worklist.pop() {
            let candidate_handle: DlHandle = candidate.lock().handle();

            // Remove the candidate from the registry only if the registry and
            // this worklist entry hold the last two references to it. A higher
            // count means another not-yet-removed dependent (e.g., the other
            // arm of a diamond dependency) or a library outside this unload set
            // still references it: skip it now and let the remaining dependent
            // re-enqueue it once that dependent is removed. An empty result
            // means the candidate was already unloaded via another path.
            let mut extracted: Vec<(DlHandle, Arc<Mutex<DynamicLibrary>>)> = registry
                .extract_if(.., |dlhandle, dlfile| {
                    *dlhandle == candidate_handle && Arc::strong_count(dlfile) == 2
                })
                .collect();

            // A handle is unique, so at most one entry can match.
            debug_assert!(extracted.len() <= 1);

            let dep: Arc<Mutex<DynamicLibrary>> = match extracted.pop() {
                Some((_, dep)) => dep,
                None => continue,
            };

            {
                let mut lib: MutexGuard<'_, DynamicLibrary> = dep.lock();
                for (_dlname, dlfile) in lib.take_dependencies() {
                    worklist.push(dlfile);
                }
                ::syslog::debug!(
                    "dlclose(): unloading dependency (name={:?}, registry.len={:?})",
                    lib.name(),
                    registry.len()
                );
            }
            fini_order.push(dep);
        }

        fini_order
    };

    // The registry lock is now released. Run `.fini_array` destructors in
    // dependents-first order with no dlfcn locks held.
    for dlfile in fini_order.iter() {
        // Snapshot the destructor descriptor and name under a short per-library
        // lock, then drop the lock before invoking destructors. Holding the
        // per-library lock during destructor execution would deadlock any
        // destructor that re-enters the same library's libposix/alloc paths.
        let (descriptor, name): (Option<(usize, usize)>, String) = {
            let lib: MutexGuard<'_, DynamicLibrary> = dlfile.lock();
            (lib.fini_array_descriptor(), String::from(lib.name()))
        };
        // SAFETY: `descriptor` was produced under the per-library lock for a
        // library still held alive by `fini_order`'s `Arc`; its segments stay
        // mapped until `fini_order` is dropped below; and no dlfcn locks are
        // held across this call, so a destructor may re-enter the loader
        // without deadlocking.
        unsafe {
            DynamicLibrary::invoke_fini_array(descriptor, &name);
        }
    }

    // Drop the libraries now that all destructors have run, unmapping their
    // segments and releasing their file descriptors.
    drop(fini_order);

    Ok(())
}
