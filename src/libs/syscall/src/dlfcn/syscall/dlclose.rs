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
    collections::{
        btree_map::BTreeMap,
        btree_set::BTreeSet,
    },
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

    // Decide which libraries are unloaded and the order in which their
    // `.fini_array` destructors must run: the closing library first, then its
    // dependencies (the reverse of constructor order). The decision is made
    // while `DYNAMIC_LIBRARY_REGISTRY` is held, but — unlike a naive
    // implementation — the entries are *not* removed and the dependency edges
    // are *not* detached here. Removal is deferred until after the destructors
    // have run (see the final phase below) so that, during teardown, the
    // libraries remain discoverable in the registry and their dependency graph
    // stays intact. This guarantees that a `.fini_array` destructor may legally
    // call back into `dlsym`/`dlopen`/`dlclose` for the library being unloaded
    // (or one of its still-loaded dependencies) and observe the existing entry
    // instead of mapping and initializing a second copy.
    let fini_order: Vec<Arc<Mutex<DynamicLibrary>>> = {
        let registry: MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>> =
            DYNAMIC_LIBRARY_REGISTRY.lock();

        // Check if dynamic library file is opened.
        if !registry.contains_key(handle) {
            let reason: &str = "dynamic library file not open";
            ::syslog::warn!("dlclose(): {}", reason);
            return Err(Error::new(ErrorCode::BadFile, reason));
        }

        // Snapshot the current strong count of every loaded library. A library
        // is unloaded once every reference to it — other than the registry's
        // own — comes from a dependent that is itself being unloaded. The
        // registry contributes exactly one reference per loaded library;
        // dependents contribute one reference per bound dependency edge;
        // libraries pinned via `RTLD_GLOBAL` carry an extra reference (held by
        // `GLOBAL_PINNED_LIBRARIES`) and therefore never reach the unload
        // threshold. The snapshot is taken before any `Arc` is cloned so that
        // these counts are not perturbed by this function.
        let base_count: BTreeMap<DlHandle, usize> = registry
            .iter()
            .map(|(dlhandle, dlfile)| (*dlhandle, Arc::strong_count(dlfile)))
            .collect();

        // Simulate the teardown without mutating the registry. `released[L]`
        // counts how many of `L`'s dependents have been scheduled for unload;
        // `L` itself becomes schedulable once `base_count[L] - released[L] == 1`
        // (i.e., only the registry still references it). A dependency that is
        // shared with a dependent outside the unload set, or that is pinned,
        // never reaches that threshold and is therefore left loaded.
        let mut released: BTreeMap<DlHandle, usize> = BTreeMap::new();
        let mut unload_set: BTreeSet<DlHandle> = BTreeSet::new();
        let mut unload_order: Vec<DlHandle> = Vec::new();
        let mut worklist: Vec<DlHandle> = ::alloc::vec![*handle];

        while let Some(candidate) = worklist.pop() {
            // Already scheduled via another path (e.g., the other arm of a
            // diamond dependency).
            if unload_set.contains(&candidate) {
                continue;
            }

            // A candidate missing from the snapshot was already unloaded by
            // another path. A candidate whose remaining references still exceed
            // the registry's own is held by a dependent outside the unload set
            // (or is pinned): skip it for now. If a remaining in-set dependent
            // later releases it, it is re-enqueued and re-examined.
            let base: usize = match base_count.get(&candidate) {
                Some(count) => *count,
                None => continue,
            };
            let rel: usize = released.get(&candidate).copied().unwrap_or(0);
            if base.saturating_sub(rel) != 1 {
                continue;
            }

            unload_set.insert(candidate);
            unload_order.push(candidate);

            // Scheduling this library releases its hold on each dependency;
            // record the release and re-examine the dependency, which may now
            // have become unreferenced.
            let dependencies: Vec<DlHandle> = match registry.get(&candidate) {
                Some(dlfile) => dlfile.lock().dependency_handles(),
                None => Vec::new(),
            };
            for dep in dependencies {
                *released.entry(dep).or_insert(0) += 1;
                worklist.push(dep);
            }
        }

        // Nothing to unload: the closing library is still referenced by another
        // loaded library (or pinned). This is not an error.
        if unload_order.is_empty() {
            return Ok(());
        }

        // Clone the scheduled libraries into the fini order, keeping them alive
        // for the destructor phase. Cloning here (after the decision) is safe:
        // the snapshot counts above are no longer consulted.
        unload_order
            .iter()
            .filter_map(|dlhandle| registry.get(dlhandle).cloned())
            .collect()
    };

    // The registry lock is now released, and the scheduled libraries are still
    // present in the registry with their dependency edges intact. Run
    // `.fini_array` destructors in dependents-first order with no dlfcn locks
    // held.
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

    // All destructors have run. Now remove the unloaded libraries from the
    // registry under the registry lock and detach their dependency edges, in
    // dependents-first order. Detaching releases each library's hold on its
    // dependencies so that, once the `fini_order` clones are dropped below, the
    // teardown cascades across the whole unload set.
    {
        let mut registry: MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>> =
            DYNAMIC_LIBRARY_REGISTRY.lock();
        for dlfile in fini_order.iter() {
            let dlhandle: DlHandle = {
                let mut lib: MutexGuard<'_, DynamicLibrary> = dlfile.lock();
                let dlhandle: DlHandle = lib.handle();
                // Detach this library's dependency edges so it releases its
                // hold on its dependencies. The returned `Arc`s are dropped
                // here; combined with removing this library from the registry
                // and dropping `fini_order` below, this lets the dependencies
                // be reclaimed.
                drop(lib.take_dependencies());
                dlhandle
            };
            registry.remove(&dlhandle);
        }
    }

    // Drop the libraries now that all destructors have run and the entries have
    // been removed from the registry, unmapping their segments and releasing
    // their file descriptors.
    drop(fini_order);

    Ok(())
}
