// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Client-side file-descriptor resolution cache.
//!
//! In standalone mode a descriptor's number encodes the backend that serves it: `0`/`1`/`2` are the
//! console (kernel), the high `[VFS_FD_BASE, …)` range is `vfsd`, and `[SOCKET_FD_BASE, …)` is
//! `networkd`. Every descriptor system call therefore decided where to route by inspecting the
//! number directly.
//!
//! This module memoizes that decision behind a single seam, [`resolve`], so that routing is asked
//! for rather than recomputed inline at each call site. The cache maps a descriptor to its
//! [`Route`] and the descriptor number the route's backend expects ([`Resolution::backend_fd`]),
//! together with the `vfsd` table generation the entry was learned at (its *epoch*).
//!
//! The cache is **behavior-preserving** here: every entry agrees with the number rules it is seeded
//! from (see [`derive`]), so a resolution answers exactly as the old number-keyed code did. The
//! epoch is plumbed and checked ([`is_coherent`]) but inert — because the route still follows the
//! number, no generation can change a routing decision. It is the coherence substrate a later plan
//! activates once descriptor numbers stop encoding their backend and a stale entry could otherwise
//! route I/O to the wrong place.

use ::alloc::collections::BTreeMap;
use ::config::fds::{
    is_socket_fd,
    is_vfs_fd,
};
use ::spin::Mutex;
use ::sysapi::unistd::{
    STDERR_FILENO,
    STDIN_FILENO,
    STDOUT_FILENO,
};

//==================================================================================================
// Structures
//==================================================================================================

/// The backend that serves a descriptor's operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    /// A console stream (`stdin`/`stdout`/`stderr`); I/O flows directly to the kernel.
    Console,
    /// A `vfsd`-managed object: regular file, directory, host file, or pipe end.
    Vfs,
    /// A `networkd`-managed socket.
    Socket,
}

/// A resolved routing decision: which backend serves a descriptor and the descriptor number that
/// backend expects.
///
/// In this plan `backend_fd` always equals the descriptor passed to [`resolve`], because numbers
/// are not yet remapped onto a flat namespace; it is carried so call sites already address their
/// backend through it when a later plan makes the two diverge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Resolution {
    /// The backend that serves the descriptor.
    pub route: Route,
    /// The descriptor number the backend expects.
    pub backend_fd: i32,
}

/// A cached resolution together with the `vfsd` table generation it was learned at.
#[derive(Debug, Clone, Copy)]
struct CacheEntry {
    /// The backend that serves the descriptor.
    route: Route,
    /// The descriptor number the backend expects.
    backend_fd: i32,
    /// The `vfsd` table generation this entry was learned at (its coherence epoch).
    epoch: u64,
}

//==================================================================================================
// Cache
//==================================================================================================

/// The process-wide resolution cache, keyed by descriptor number.
///
/// Guarded by a [`spin::Mutex`] held only for the duration of a single map operation so the hot
/// `read`/`write` path is never blocked on anything but a peer lookup, and never allocates on a
/// resolve.
static CACHE: Mutex<BTreeMap<i32, CacheEntry>> = Mutex::new(BTreeMap::new());

/// Derives the routing decision implied by a descriptor number alone.
///
/// This encodes the number rules that governed routing before the cache existed and is what every
/// cache entry is seeded from, so a resolve answers identically whether it hits the cache or falls
/// back here. Returns `None` for a number that belongs to no backend range.
fn derive(fd: i32) -> Option<Resolution> {
    if fd == STDIN_FILENO || fd == STDOUT_FILENO || fd == STDERR_FILENO {
        Some(Resolution {
            route: Route::Console,
            backend_fd: fd,
        })
    } else if is_vfs_fd(fd) {
        Some(Resolution {
            route: Route::Vfs,
            backend_fd: fd,
        })
    } else if is_socket_fd(fd) {
        Some(Resolution {
            route: Route::Socket,
            backend_fd: fd,
        })
    } else {
        None
    }
}

/// Reports whether a cache entry learned at `_entry_epoch` is still coherent with the authoritative
/// `vfsd` table.
///
/// Inert in this plan: a descriptor's route is implied by its number, which never goes stale, so an
/// entry is coherent regardless of the generation it was learned at. A later plan replaces this with
/// a comparison against the live `vfsd` generation, at which point the epoch becomes load-bearing
/// and a mismatch forces re-resolution.
fn is_coherent(_entry_epoch: u64) -> bool {
    true
}

/// Resolves a descriptor to its backend route.
///
/// This is the hot-path lookup used by every descriptor system call. A cached entry is returned
/// when present and coherent; otherwise the decision is derived from the descriptor number. The
/// lookup is allocation-free and holds the cache lock only across a single map probe.
pub(crate) fn resolve(fd: i32) -> Option<Resolution> {
    {
        let cache: ::spin::MutexGuard<'_, BTreeMap<i32, CacheEntry>> = CACHE.lock();
        if let Some(entry) = cache.get(&fd) {
            if is_coherent(entry.epoch) {
                return Some(Resolution {
                    route: entry.route,
                    backend_fd: entry.backend_fd,
                });
            }
        }
    }
    derive(fd)
}

/// Records the resolution learned for `fd` from a backend response, stamped with the `epoch` the
/// backend reported.
///
/// Called when a descriptor is created (`open`/`socket`) so the cache holds an authoritative entry
/// rather than relying on re-derivation. An existing entry for `fd` is replaced.
pub(crate) fn record(fd: i32, route: Route, backend_fd: i32, epoch: u64) {
    CACHE.lock().insert(
        fd,
        CacheEntry {
            route,
            backend_fd,
            epoch,
        },
    );
}

/// Drops any cached resolution for `fd`.
///
/// Called when a descriptor is destroyed (`close`) so a number later reused by a different backend
/// cannot be answered from a stale entry.
pub(crate) fn invalidate(fd: i32) {
    CACHE.lock().remove(&fd);
}

/// Drops every cached resolution.
#[cfg(test)]
fn clear() {
    CACHE.lock().clear();
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::config::fds::{
        SOCKET_FD_BASE,
        VFS_FD_BASE,
    };

    /// Serializes the cache tests: they share the process-global [`CACHE`], so they must not run
    /// concurrently with one another.
    static CACHE_TEST_GUARD: Mutex<()> = Mutex::new(());

    /// Tests that a descriptor with no cached entry resolves by the number rules.
    #[test]
    fn derive_seeds_from_number_rules() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        assert_eq!(
            resolve(STDIN_FILENO),
            Some(Resolution {
                route: Route::Console,
                backend_fd: STDIN_FILENO
            }),
            "stdin must route to the console"
        );
        assert_eq!(
            resolve(STDOUT_FILENO).map(|r| r.route),
            Some(Route::Console),
            "stdout must route to the console"
        );
        assert_eq!(
            resolve(STDERR_FILENO).map(|r| r.route),
            Some(Route::Console),
            "stderr must route to the console"
        );
        assert_eq!(
            resolve(VFS_FD_BASE),
            Some(Resolution {
                route: Route::Vfs,
                backend_fd: VFS_FD_BASE
            }),
            "a high descriptor must route to vfsd"
        );
        assert_eq!(
            resolve(SOCKET_FD_BASE).map(|r| r.route),
            Some(Route::Socket),
            "a socket descriptor must route to networkd"
        );
        assert_eq!(resolve(7), None, "a descriptor in no backend range is unroutable");

        clear();
    }

    /// Tests that a recorded entry is returned by `resolve`, overriding number-rule derivation.
    #[test]
    fn record_then_resolve_hits_cache() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        // A number that derives to nothing still resolves once recorded, proving the cache — not the
        // number — is consulted first, and that `backend_fd` may differ from the descriptor.
        record(7, Route::Vfs, VFS_FD_BASE + 5, 1);
        assert_eq!(
            resolve(7),
            Some(Resolution {
                route: Route::Vfs,
                backend_fd: VFS_FD_BASE + 5
            }),
            "a recorded entry must win over number-rule derivation"
        );

        clear();
    }

    /// Tests that `invalidate` drops a single entry, after which resolution falls back to the number
    /// rules.
    #[test]
    fn invalidate_drops_entry() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        record(VFS_FD_BASE, Route::Vfs, VFS_FD_BASE, 3);
        invalidate(VFS_FD_BASE);
        // Re-derives to the same answer here, but from the number rather than the dropped entry.
        assert_eq!(
            resolve(VFS_FD_BASE).map(|r| r.route),
            Some(Route::Vfs),
            "after invalidation the descriptor re-derives from its number"
        );
        // A recorded-only number reverts to unroutable once invalidated.
        record(7, Route::Socket, 7, 3);
        invalidate(7);
        assert_eq!(resolve(7), None, "an invalidated recorded-only entry is gone");

        clear();
    }

    /// Tests that `clear` drops every entry.
    #[test]
    fn clear_drops_all() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        record(7, Route::Vfs, 7, 1);
        record(8, Route::Socket, 8, 1);
        clear();
        assert_eq!(resolve(7), None, "clear must drop recorded entries");
        assert_eq!(resolve(8), None, "clear must drop recorded entries");

        clear();
    }

    /// Tests that the epoch is inert: a descriptor's route does not depend on the generation its
    /// entry was learned at.
    #[test]
    fn epoch_does_not_change_routing() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        record(VFS_FD_BASE, Route::Vfs, VFS_FD_BASE, 1);
        let early: Option<Route> = resolve(VFS_FD_BASE).map(|r| r.route);
        // Re-learn the same descriptor at a much newer generation.
        record(VFS_FD_BASE, Route::Vfs, VFS_FD_BASE, 9_999);
        let late: Option<Route> = resolve(VFS_FD_BASE).map(|r| r.route);
        assert_eq!(early, Some(Route::Vfs));
        assert_eq!(early, late, "bumping the epoch must not change the routing decision");

        clear();
    }
}
