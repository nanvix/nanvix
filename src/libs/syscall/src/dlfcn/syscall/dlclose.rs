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

    // Phase 1: determine, WITHOUT mutating the registry, the set of libraries
    // that become unreferenced when `handle` is closed, ordered dependents-
    // first (the reverse of constructor order). Entries are left in the
    // registry and their dependency edges left intact: while the `.fini_array`
    // destructors run below (phase 2), the libraries therefore stay
    // discoverable -- a re-entrant `dlopen` of one of them returns the existing
    // entry instead of mapping a second copy, and `dlsym(handle, ...)` keeps
    // resolving symbols across the still-connected dependency graph. The
    // registry removal is deferred to phase 3, after every destructor has run.
    let fini_order: Vec<Arc<Mutex<DynamicLibrary>>> = {
        let registry: MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>> =
            DYNAMIC_LIBRARY_REGISTRY.lock();

        // Check if the dynamic library file is open.
        let root: &Arc<Mutex<DynamicLibrary>> = match registry.get(handle) {
            Some(root) => root,
            None => {
                let reason: &str = "dynamic library file not open";
                ::syslog::warn!("dlclose(): {}", reason);
                return Err(Error::new(ErrorCode::BadFile, reason));
            },
        };

        // Balance this handle's `dlopen()`. Per POSIX the library only becomes a
        // candidate for unloading once every direct `dlopen()` has been matched
        // by a `dlclose()`; while any handle is still live, keep it loaded. This
        // also covers cache-hit opens that share a handle, which do not each hold
        // their own `Arc`.
        if root.lock().decrement_open_count() > 0 {
            return Ok(());
        }

        // The closing library can only be unloaded if the registry holds the
        // last reference to it. A higher count means another loaded library
        // still depends on it, or it is pinned in the global scope
        // (`RTLD_GLOBAL`): there is nothing to unload yet.
        if Arc::strong_count(root) != 1 {
            return Ok(());
        }

        // Reference-count peel over the dependency graph. `remaining[h]` is the
        // number of references to `h` that will survive once every library
        // already placed in the unload set is torn down; a library joins the
        // set when that count falls to just the registry's own reference (`1`).
        // Neither the registry nor any dependency edge is mutated here, and the
        // owning `Arc`s are cloned only afterwards, below. A live
        // `Arc::strong_count` may still change concurrently (e.g. another
        // thread completing a dlopen/dlclose), so this peel only computes a
        // candidate unload set; phase 3 re-checks ownership before removing any
        // entry.
        let mut remaining: BTreeMap<DlHandle, usize> = BTreeMap::new();
        let mut visited: BTreeSet<DlHandle> = BTreeSet::new();
        let mut order: Vec<DlHandle> = Vec::new();
        let mut stack: Vec<DlHandle> = Vec::new();

        visited.insert(*handle);
        order.push(*handle);
        stack.push(*handle);

        while let Some(parent) = stack.pop() {
            // Snapshot the parent's bound dependency handles under a short lock.
            let dependencies: Vec<DlHandle> = match registry.get(&parent) {
                Some(dlfile) => dlfile.lock().dependency_handles(),
                None => continue,
            };

            for dependency in dependencies {
                // Seed `remaining` from the dependency's live reference count
                // the first time it is reached, then release the edge that
                // `parent` -- now in the unload set -- holds on it.
                let count: &mut usize = remaining.entry(dependency).or_insert_with(|| {
                    registry
                        .get(&dependency)
                        .map(Arc::strong_count)
                        .unwrap_or(1)
                });
                *count = count.saturating_sub(1);

                // Once only the registry still references the dependency, it too
                // becomes part of the unload set. A dependency shared with a
                // library outside the set (or pinned in the global scope) never
                // reaches `1` and is correctly left loaded. A dependency that is
                // ALSO held open by a direct `dlopen()` (its `open_count` is
                // non-zero) is likewise left loaded even once its edges are gone.
                if *count == 1
                    && registry
                        .get(&dependency)
                        .map(|dep| dep.lock().open_count())
                        .unwrap_or(0)
                        == 0
                    && visited.insert(dependency)
                {
                    order.push(dependency);
                    stack.push(dependency);
                }
            }
        }

        // Collect owning references in dependents-first order. Cloning here
        // (after the peel) keeps every library alive across the destructor
        // phase and, crucially, bumps each `strong_count` above `1`, so a
        // destructor that re-enters `dlclose` for one of these handles sees it
        // as still referenced and does not unload it a second time.
        order
            .iter()
            .filter_map(|handle| registry.get(handle).cloned())
            .collect()
    };

    // Phase 2: the registry lock is released. Run `.fini_array` destructors in
    // dependents-first order with no dlfcn locks held, so a destructor may
    // legally call back into `dlsym`/`dlopen`/`dlclose`.
    for dlfile in fini_order.iter() {
        // Snapshot the destructor descriptor and name under a short per-library
        // lock, then drop the lock before invoking destructors. Holding the
        // per-library lock during destructor execution would deadlock any
        // destructor that re-enters the same library's libposix/alloc paths, or
        // that calls `dlsym(self_handle, ...)`.
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

    // Phase 3: every destructor has run. Re-acquire the registry lock and remove
    // the unloaded libraries that are still owned only by the registry and this
    // `fini_order` snapshot, detaching their dependency edges so the owning
    // `Arc`s are released. A destructor may have re-entered `dlopen` and added a
    // new real reference to one of these libraries (for example by loading a new
    // dependent or promoting it to `RTLD_GLOBAL`); such libraries must remain in
    // the registry with their dependency edges intact. Detaching a shared
    // dependency's edge that is kept alive by a library OUTSIDE the unload set
    // correctly decrements that dependency's reference count without unloading
    // it. The actual unmap happens when `fini_order` is dropped below, with the
    // registry lock NOT held (segment teardown issues kernel calls).
    {
        let mut registry: MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>> =
            DYNAMIC_LIBRARY_REGISTRY.lock();
        for dlfile in fini_order.iter() {
            let handle: DlHandle = dlfile.lock().handle();
            if !registry.contains_key(&handle) {
                continue;
            }

            // A destructor (phase 2) may have re-opened this library through a
            // cache-hit `dlopen()`, which bumps its direct open count without
            // adding a lasting `Arc`. Honor that open like the entry guard does
            // and leave the library loaded.
            if dlfile.lock().open_count() > 0 {
                ::syslog::debug!(
                    "dlclose(): keeping re-opened library loaded (handle={:?})",
                    handle
                );
                continue;
            }

            let strong_count: usize = Arc::strong_count(dlfile);
            if strong_count != 2 {
                ::syslog::debug!(
                    "dlclose(): keeping library loaded (handle={:?}, references={:?})",
                    handle,
                    strong_count
                );
                continue;
            }

            if registry.remove(&handle).is_some() {
                ::syslog::debug!(
                    "dlclose(): unloading library (handle={:?}, registry.len={:?})",
                    handle,
                    registry.len()
                );
                dlfile.lock().take_dependencies();
            }
        }
    }

    // Drop the libraries now that all destructors have run and they have been
    // removed from the registry, unmapping their segments and releasing their
    // file descriptors. The registry lock is not held here.
    drop(fini_order);

    Ok(())
}
