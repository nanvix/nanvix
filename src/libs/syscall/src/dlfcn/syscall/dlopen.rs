// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use super::dynlib::{
    DlHandle,
    DynamicLibrary,
    InitState,
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
use ::sys::error::{
    Error,
    ErrorCode,
};

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
    // Paths that already contain a separator are passed through with only a
    // leading "./" stripped; a leading "/" is preserved so that absolute
    // paths stay absolute and resolve against the filesystem root regardless
    // of the caller's CWD. As a consequence, "/lib/libc.so" and
    // "lib/libc.so" are distinct keys in the already-loaded-library lookup
    // below (matching the as-supplied path rather than a canonical form).
    //
    // A direct `dlopen()` has no loading-library context, so no `DT_RUNPATH`
    // entries are supplied here; only the configured default search paths are
    // consulted for bare names.
    let resolved: String = super::resolve_library_path(filename, None);
    let filename: &str = resolved.as_str();

    let mut registry: MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>> =
        DYNAMIC_LIBRARY_REGISTRY.lock();

    // Check if dynamic library is already opened.
    let mut already_loaded: Option<(DlHandle, Arc<Mutex<DynamicLibrary>>)> = None;
    for (dlhandle, dlfile) in registry.iter() {
        if dlfile.lock().name() == filename {
            already_loaded = Some((*dlhandle, dlfile.clone()));
            break;
        }
    }
    if let Some((dlhandle, dlfile)) = already_loaded {
        // If the caller requests RTLD_GLOBAL on a library that was
        // previously loaded without it, promote it to global scope now.
        if global {
            super::register_library_in_global_scope(&dlfile);
        }

        // Record this additional direct open of an already-loaded library so a
        // later `dlclose()` of one handle does not unload it while another
        // handle is still live.
        dlfile.lock().increment_open_count();

        // A concurrent `dlopen` may have inserted this library and released the
        // registry lock but not yet finished running its `.init_array`
        // constructors. Wait for them to complete so this caller never observes
        // a handle to an uninitialized library.
        //
        // The registry lock is dropped BEFORE waiting: the constructing thread
        // needs it to service the constructor's own `dlsym`/`dlopen` calls, so
        // holding it here would deadlock them. A re-entrant open from the
        // constructing thread itself does not wait — see
        // `InitState::wait_until_constructed`.
        let tid: i32 = current_tid()?;
        let init_state: Arc<InitState> = dlfile.lock().init_state();
        drop(registry);
        init_state.wait_until_constructed(tid);
        return Ok(dlhandle);
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
    let init_order: Vec<Arc<Mutex<DynamicLibrary>>> =
        match load_all_dependencies(&mut registry, new_dlfile)
            .and_then(|_| resolve_all_symbols(&mut registry, &handles_before))
        {
            Ok(order) => {
                // If RTLD_GLOBAL was requested, publish the library's exported
                // symbols into the global symbol table so subsequently loaded
                // libraries can resolve them.
                if global {
                    if let Some(dlfile) = registry.get(&handle) {
                        super::register_library_in_global_scope(dlfile);
                    }
                }
                order
            },
            Err(e) => {
                let new_handles: Vec<DlHandle> = registry
                    .keys()
                    .filter(|h| !handles_before.contains(h))
                    .copied()
                    .collect();
                ::syslog::warn!(
                    "dlopen(): rolling back {} entries after failure (error={:?})",
                    new_handles.len(),
                    e
                );
                for h in new_handles {
                    registry.remove(&h);
                }
                return Err(e);
            },
        };

    // Record this direct `dlopen()` on the freshly loaded root library. Only
    // the library named by the caller is counted; dependencies loaded
    // transitively are tracked by their `Arc` edges, not by this direct open
    // count.
    if let Some(dlfile) = registry.get(&handle) {
        dlfile.lock().increment_open_count();
    }

    // Record the constructing thread on every newly loaded library BEFORE
    // releasing the registry lock. A concurrent `dlopen` that observes any of
    // them on the dedup path above then either waits for their constructors
    // (a different thread) or skips the wait (this same thread re-entering from
    // one of the constructors). Publishing the thread id under the registry
    // lock guarantees no concurrent observer can see the pre-set sentinel as a
    // constructor owner.
    let constructor_tid: i32 = current_tid()?;
    for dlfile in init_order.iter() {
        dlfile
            .lock()
            .init_state()
            .set_constructor_thread(constructor_tid);
    }

    // Drop the registry lock before invoking `.init_array` constructors so a
    // constructor may legally call `dlsym` (and, in a future relaxation,
    // `dlopen`) without deadlocking on `DYNAMIC_LIBRARY_REGISTRY`.
    drop(registry);

    // Invoke `.init_array` constructors in dependency order (leaves first).
    // The Arc list was built while the registry was locked, so each entry is
    // guaranteed to still point to a loaded library.
    //
    // Snapshot the constructor descriptor and library name under a short
    // per-library lock, then drop the lock before invoking constructors.
    // Holding the per-library lock during constructor execution would
    // deadlock any constructor that calls `dlsym(self_handle, ...)`, because
    // `dlsym` re-locks the same library to look up symbols.
    for dlfile in init_order.iter() {
        let (descriptor, name, init_state): (Option<(usize, usize)>, String, Arc<InitState>) = {
            let lib: MutexGuard<'_, DynamicLibrary> = dlfile.lock();
            (lib.init_array_descriptor(), String::from(lib.name()), lib.init_state())
        };
        // SAFETY: `descriptor` was produced under the per-library lock for
        // a library still held alive by `init_order`'s `Arc`; relocations
        // have been applied by `resolve_all_symbols`; no dlfcn locks are
        // held across this call.
        unsafe {
            DynamicLibrary::invoke_init_array(descriptor, &name);
        }
        // Publish completion (leaves first) so a concurrent `dlopen` blocked in
        // the dedup path above may now return this library's handle. Marking
        // per-library — rather than once after the whole batch — lets a waiter
        // for a dependency proceed as soon as that dependency is constructed,
        // without waiting for its dependents.
        init_state.mark_constructed();
    }

    Ok(handle)
}

/// Returns the raw identifier of the calling thread.
///
/// The identifier is mandatory: it is the only way to tell a re-entrant
/// `dlopen` issued by a constructor running on the loading thread apart from a
/// genuine concurrent open on another thread. Without it, such a re-entrant
/// call would block forever in [`InitState::wait_until_constructed`] waiting for
/// constructors that cannot finish until this very call returns. A lookup
/// failure is therefore propagated to the caller rather than silently ignored.
fn current_tid() -> Result<i32, Error> {
    Ok(i32::from(::sys::kcall::pm::__kcall_gettid()?))
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
        ancestors: &mut BTreeMap<DlHandle, String>,
    ) -> Result<(), Error> {
        // Snapshot the loader's `DT_RUNPATH` entries so they are visible to
        // `resolve_library_path` while probing every dependency below. The
        // `DynamicLibrary` mutex is borrowed throughout, so this avoids
        // re-borrowing it per call.
        let runpaths: Vec<String> = new_dlfile.runpaths().to_vec();

        // Snapshot the current library's own name so it can be (a) recorded as
        // an ancestor for the recursive frames below and (b) compared against
        // each dependency to detect a `DT_NEEDED` self-cycle, both without
        // re-locking the (already held) `DynamicLibrary` mutex.
        let self_name: String = new_dlfile.name().to_string();

        // Collect the name of all dependencies, in DT_NEEDED order.
        let mut dependencies: Vec<String> = new_dlfile
            .dependencies()
            .into_iter()
            .map(|(dlname, _)| dlname)
            .collect();

        // Bind to already loaded dependencies and remove them from the list.
        //
        // The closure below calls `dlfile.lock()` on every other entry in the
        // registry while looking for a matching name. Those locks must skip
        // BOTH the current library (`new_dlhandle`) AND every ancestor still
        // held by an outer recursive frame — otherwise any non-trivial
        // recursive load (e.g. `libA → libB`, and especially diamond graphs
        // like the one below) deadlocks:
        //
        //   dlopen(libdiamond.so)                  // holds libdiamond lock
        //     -> recurse into libright.so          // also holds libright lock
        //          -> retain() iterates dlfiles    // tries to lock libdiamond
        //             ^ blocks forever, libdiamond is still held by the
        //               outer frame.
        //
        // What this `retain` legitimately consolidates is a dependency that a
        // PRIOR, already-finished iteration of an OUTER frame loaded. In the
        // classic diamond
        //
        //   libdiamond.so -> libleft.so  -> libbase.so
        //                 -> libright.so -> libbase.so
        //
        // libdiamond finishes libleft first (recursing in and loading
        // libbase), then starts libright's frame; libright's `retain` then
        // sees libbase already resident and binds libright's `libbase` edge to
        // that single shared instance here. (The other shape — a sibling
        // pulled in LATER within the *same* frame's own dependency list — is
        // caught by the re-check in the load loop below, not here.)
        //
        // Skipping ancestors by handle is safe: the only dependency that can
        // legitimately resolve to an ancestor is a `DT_NEEDED` cycle, which is
        // detected and rejected explicitly in the load loop below (see the
        // `self_name`/`ancestors` cycle check), so it never reaches this scan.
        dependencies.retain(|dependency| {
            // Resolve bare name so we can match against loaded libraries
            // that were opened with a full path.
            let resolved_dep: String = super::resolve_library_path(dependency, Some(&runpaths));
            for (dlhandle, dlfile) in dlfiles.iter() {
                // Skip the dynamic library itself and any ancestor held by
                // an outer frame's lock — locking them would deadlock.
                if dlhandle == new_dlhandle || ancestors.contains_key(dlhandle) {
                    continue;
                }

                let is_match: bool = {
                    let loaded_file: spin::MutexGuard<'_, DynamicLibrary> = dlfile.lock();
                    let loaded_name: &str = loaded_file.name();
                    loaded_name == dependency.as_str() || loaded_name == resolved_dep
                };
                if is_match {
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

        // Load remaining dependencies in DT_NEEDED order. The loop below uses
        // `pop()` for ownership, so reverse once to consume the original front
        // of the list first.
        dependencies.reverse();
        while let Some(dependency) = dependencies.pop() {
            // Resolve bare library names to full paths using search directories.
            let resolved_dep: String = super::resolve_library_path(&dependency, Some(&runpaths));

            // Reject `DT_NEEDED` cycles before they recurse without bound. A
            // dependency that resolves to the current library or to any
            // ancestor still being loaded by an outer recursive frame cannot be
            // followed: the ancestor's mutex is held, so we can neither recurse
            // into it nor lock it to consolidate onto it, and skipping it by
            // handle (as `retain` and the re-check do) would just open a fresh
            // copy on a new handle and recurse forever. Cycles in `DT_NEEDED`
            // are pathological — no sane toolchain emits them — so reject the
            // load cleanly; `dlopen` then rolls back every entry added during
            // this call.
            if self_name == dependency
                || self_name == resolved_dep
                || ancestors
                    .values()
                    .any(|name| name == &dependency || name == &resolved_dep)
            {
                let reason: &str = "cyclic DT_NEEDED dependency";
                ::syslog::warn!(
                    "load_all_dependencies_recursive(): {} (dependency '{}')",
                    reason,
                    dependency
                );
                return Err(Error::new(ErrorCode::BadFile, reason));
            }

            // Re-check the registry before opening: an EARLIER iteration of
            // THIS frame's own load loop may have already loaded this exact
            // dependency transitively. `retain` above runs only once, at frame
            // entry, before any sibling is processed, so it cannot see a
            // sibling that a later-processed sibling pulls in. Concretely, when
            // a parent directly `DT_NEEDED`s both `libX` and `libbase` and
            // `libX -> libbase`:
            //
            //   libdiamond.so -> libright.so -> libbase.so   (processed first)
            //                 -> libbase.so                  (direct edge)
            //
            // processing `libright` loads `libbase`; when the loop reaches
            // libdiamond's own direct `libbase` edge it is already resident and
            // must be bound here, not re-opened. (dlfcn-diamond-c builds
            // libdiamond with exactly this direct `libbase` edge to exercise
            // this path.) Without this check the loader would open `libbase` a
            // second time — two distinct in-memory copies with two private
            // `unique_counter`s — or trip the `unreachable!()` below if the VFS
            // recycles the underlying file descriptor.
            let already_loaded: Option<Arc<Mutex<DynamicLibrary>>> =
                dlfiles.iter().find_map(|(dlhandle, dlfile)| {
                    if dlhandle == new_dlhandle || ancestors.contains_key(dlhandle) {
                        return None;
                    }
                    let loaded_file: spin::MutexGuard<'_, DynamicLibrary> = dlfile.lock();
                    let loaded_name: &str = loaded_file.name();
                    if loaded_name == dependency.as_str() || loaded_name == resolved_dep {
                        Some(dlfile.clone())
                    } else {
                        None
                    }
                });
            if let Some(existing) = already_loaded {
                ::syslog::debug!(
                    "load_all_dependencies_recursive(): dependency '{}' loaded transitively \
                     during this dlopen call; binding to existing copy",
                    dependency
                );
                new_dlfile.bind_dependency(dependency, existing)?;
                continue;
            }

            // Open and pre-load the dynamic library file.
            let dep_dlfile: DynamicLibrary = DynamicLibrary::open(&resolved_dep)?;
            let handle: DlHandle = dep_dlfile.handle();
            let dep_dlfile: Arc<Mutex<DynamicLibrary>> = Arc::new(Mutex::new(dep_dlfile));

            // Insert the opened file into the map.
            if let Some(dlfile) = dlfiles.insert(handle, dep_dlfile.clone()) {
                unreachable!("dlopen(): library file already loaded (dlfile={:?})", dlfile);
            }

            new_dlfile.bind_dependency(dependency.clone(), dep_dlfile.clone())?;

            // Load dependencies of the new dynamic library file. Record the
            // current library as an ancestor (keyed by handle, valued by its
            // name) so the recursive frame neither tries to lock our still-held
            // mutex (would deadlock on diamond DT_NEEDED graphs) nor mistakes a
            // cycle back to us for a brand-new library.
            ancestors.insert(*new_dlhandle, self_name.clone());
            let mut dlfile: MutexGuard<'_, DynamicLibrary> = dep_dlfile.lock();
            let result = load_all_dependencies_recursive(dlfiles, &handle, &mut dlfile, ancestors);
            ancestors.remove(new_dlhandle);
            result?;
        }

        Ok(())
    }

    let mut new_dlfile = new_dlfile.lock();
    let new_dlhandle = new_dlfile.handle();
    let mut ancestors: BTreeMap<DlHandle, String> = BTreeMap::new();
    load_all_dependencies_recursive(dlfiles, &new_dlhandle, &mut new_dlfile, &mut ancestors)?;

    Ok(())
}

/// Resolves all relocations for libraries added during this `dlopen` call and
/// returns them in dependency order (leaves first, root last) so the caller
/// can invoke `.init_array` constructors in the correct sequence.
///
/// Libraries are resolved in dependency order. This ensures that when a
/// library's relocations reference symbols from its dependencies, those
/// dependencies are already fully resolved.
fn resolve_all_symbols(
    dlfiles: &mut MutexGuard<'_, BTreeMap<DlHandle, Arc<Mutex<DynamicLibrary>>>>,
    handles_before: &BTreeSet<DlHandle>,
) -> Result<Vec<Arc<Mutex<DynamicLibrary>>>, Error> {
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
    // This attempts a simple topological ordering for the newly loaded
    // libraries. If the dependency graph contains a cycle, or if a dependency
    // cannot be ordered, the no-progress fallback below will warn and resolve
    // the remaining libraries in arbitrary order.
    loop {
        let mut progress: bool = false;
        for &handle in &new_handles {
            if resolved.contains(&handle) {
                continue;
            }

            // Check if all of this library's dependencies that are new in
            // this dlopen call have been resolved.
            let dlfile: &Arc<Mutex<DynamicLibrary>> = match dlfiles.get(&handle) {
                Some(f) => f,
                None => {
                    // Handle came from dlfiles.keys(), so this should not happen.
                    ::syslog::warn!(
                        "resolve_all_symbols(): handle {:?} missing from registry",
                        handle
                    );
                    continue;
                },
            };
            // Lock the library only long enough to check its dependencies.
            let all_deps_resolved: bool = {
                let lib: spin::MutexGuard<'_, DynamicLibrary> = dlfile.lock();
                lib.dependency_handles().iter().all(|dep_handle| {
                    // Dependencies that existed before this dlopen are already resolved.
                    handles_before.contains(dep_handle) || resolved.contains(dep_handle)
                })
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
    let mut init_order: Vec<Arc<Mutex<DynamicLibrary>>> = Vec::with_capacity(ordered.len());
    for handle in ordered {
        if let Some(dlfile) = dlfiles.get(&handle) {
            dlfile.lock().resolve_all()?;
            init_order.push(dlfile.clone());
        }
    }

    Ok(init_order)
}
