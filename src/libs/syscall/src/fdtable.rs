// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Client-side file-descriptor resolution cache.
//!
//! Under the flat namespace a descriptor's number no longer encodes its backend: `open` hands out
//! the lowest free number (typically a small one), so a value like `4` could be a regular file, a
//! pipe end, a directory, a host file, or a socket. Every descriptor is whatever `vfsd`'s
//! authoritative slot table says it is.
//!
//! This module routes a descriptor through a single seam, [`resolve`]. The answer comes from, in
//! order: a coherent cache entry; otherwise an authoritative query to `vfsd` ([`resolve_via_vfsd`]).
//! If no guest `vfsd` exists in the current run mode, the standard streams fall back to the kernel
//! console by number. A descriptor created locally (`open`, `pipe`, `socket`) or learned from
//! `vfsd` is recorded, so the `vfsd` query is reached only on a genuine miss or after an entry goes
//! stale.
//!
//! Coherence is load-bearing here. Each entry carries the `vfsd` table generation it was learned at
//! (its *epoch*), and [`EXPECTED_EPOCH`] tracks the newest generation this process has observed from
//! `vfsd`. An entry older than that ([`is_coherent`]) is treated as stale and re-resolved, so a
//! number reused for a different backend can never be answered from an outdated entry.

#[cfg(any(feature = "standalone", test))]
use ::alloc::collections::BTreeMap;
#[cfg(test)]
use ::core::sync::atomic::AtomicBool;
#[cfg(any(feature = "standalone", test))]
use ::core::sync::atomic::{
    AtomicU64,
    Ordering,
};
#[cfg(any(feature = "standalone", test))]
use ::spin::Mutex;
use ::sys::error::Error;
#[cfg(any(feature = "standalone", test))]
use ::sysapi::unistd::{
    STDERR_FILENO,
    STDIN_FILENO,
    STDOUT_FILENO,
};

//==================================================================================================
// Structures
//==================================================================================================

/// The backend that serves a descriptor's operations.
#[cfg(any(feature = "standalone", test))]
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
/// `backend_fd` may differ from the descriptor passed to [`resolve`]: a console descriptor reports
/// the standard stream number it aliases (`0`/`1`/`2`), and a socket reports the descriptor
/// `networkd` assigned. Call sites address their backend through `backend_fd` rather than the
/// caller-facing number.
#[cfg(any(feature = "standalone", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Resolution {
    /// The backend that serves the descriptor.
    pub route: Route,
    /// The descriptor number the backend expects.
    pub backend_fd: i32,
}

/// A cached resolution together with the `vfsd` table generation it was learned at.
#[cfg(any(feature = "standalone", test))]
#[derive(Debug, Clone, Copy)]
struct CacheEntry {
    /// The backend that serves the descriptor.
    route: Route,
    /// The descriptor number the backend expects.
    backend_fd: i32,
    /// The `vfsd` table generation this entry was learned at (its coherence epoch).
    epoch: u64,
}

/// Outcome of asking `vfsd` for an authoritative descriptor route.
#[cfg(any(feature = "standalone", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VfsdResolution {
    /// `vfsd` returned an authoritative route.
    Hit(Resolution),
    /// `vfsd` answered and reported that the descriptor has no slot.
    BadFile,
    /// The query could not be delivered or answered because no `vfsd` is available.
    Unavailable,
}

//==================================================================================================
// Cache
//==================================================================================================

/// The process-wide resolution cache, keyed by descriptor number.
///
/// Guarded by a [`spin::Mutex`] held only for the duration of a single map operation so the hot
/// `read`/`write` path is never blocked on anything but a peer lookup, and never allocates on a
/// resolve.
#[cfg(any(feature = "standalone", test))]
static CACHE: Mutex<BTreeMap<i32, CacheEntry>> = Mutex::new(BTreeMap::new());

/// The newest `vfsd` table generation this process has observed.
///
/// Every `vfsd` answer that carries a generation ([`record`] from `open`/`pipe`, and the
/// resolution query) advances this through [`observe_epoch`]. A cache entry learned at an older
/// generation is stale ([`is_coherent`]) and must be re-resolved, because the descriptor it
/// describes may since have been closed and the number reused for a different backend.
#[cfg(any(feature = "standalone", test))]
static EXPECTED_EPOCH: AtomicU64 = AtomicU64::new(0);

/// Records that `vfsd` has advanced its table generation to at least `epoch`.
///
/// Monotonic via a max, so an out-of-order or duplicated response can never move the observed
/// generation backwards and resurrect a stale entry.
#[cfg(any(feature = "standalone", test))]
fn observe_epoch(epoch: u64) {
    EXPECTED_EPOCH.fetch_max(epoch, Ordering::Relaxed);
}

/// Derives the fallback console route for run modes that have no guest `vfsd`.
#[cfg(any(feature = "standalone", test))]
fn derive_console(fd: i32) -> Option<Resolution> {
    if fd == STDIN_FILENO || fd == STDOUT_FILENO || fd == STDERR_FILENO {
        Some(Resolution {
            route: Route::Console,
            backend_fd: fd,
        })
    } else {
        None
    }
}

/// Reports whether a cache entry learned at `entry_epoch` is still coherent with the authoritative
/// `vfsd` table.
///
/// An entry is coherent only if it was learned no earlier than the newest generation this process
/// has observed from `vfsd`. A descriptor-table mutation advances that generation, so an entry that
/// predates the latest one is treated as stale and re-resolved, guaranteeing a reused number is
/// never answered from an entry that described its previous occupant.
#[cfg(any(feature = "standalone", test))]
fn is_coherent(entry_epoch: u64) -> bool {
    entry_epoch >= EXPECTED_EPOCH.load(Ordering::Relaxed)
}

/// Resolves a descriptor to its backend route.
///
/// This is the hot-path lookup used by every descriptor system call. The answer comes from, in
/// order: a cached entry that is still coherent; otherwise an authoritative query to `vfsd`
/// ([`resolve_via_vfsd`]).
/// A cache hit on a coherent entry holds the lock only across a single map probe and never
/// round-trips, so tight `read`/`write` loops stay local.
#[cfg(any(feature = "standalone", test))]
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
    // The cache missed or went stale; the authority decides, falling back to the console by number
    // only when no guest `vfsd` is available to answer.
    match resolve_via_vfsd(fd) {
        VfsdResolution::Hit(resolution) => Some(resolution),
        VfsdResolution::BadFile => None,
        VfsdResolution::Unavailable => derive_console(fd),
    }
}

/// Resolves `fd` and returns the descriptor expected by `vfsd`.
///
/// In standalone mode, fd-taking VFS syscalls must reject console, socket, and invalid descriptors
/// rather than sending the caller-facing number directly to `vfsd`. In hosted mode descriptors are
/// still interpreted by `linuxd`, so the raw descriptor is already the backend descriptor.
#[cfg(feature = "standalone")]
pub(crate) fn resolve_vfs(fd: i32, syscall_name: &str) -> Result<i32, Error> {
    use ::sys::error::ErrorCode;

    match resolve(fd) {
        Some(resolution) if resolution.route == Route::Vfs => Ok(resolution.backend_fd),
        _ => {
            ::syslog::warn!("{syscall_name}(): bad file descriptor fd={fd}");
            Err(Error::new(ErrorCode::BadFile, "fd is not a VFS fd"))
        },
    }
}

/// Resolves `fd` and returns the descriptor expected by the VFS backend.
///
/// Non-standalone builds route these syscalls to `linuxd`, which interprets the caller-facing
/// descriptor directly.
#[cfg(not(feature = "standalone"))]
pub(crate) fn resolve_vfs(fd: i32, _syscall_name: &str) -> Result<i32, Error> {
    Ok(fd)
}

/// Resolves `fd` and returns the `networkd` descriptor backing the socket.
///
/// Socket I/O syscalls take a flat descriptor but must address `networkd` by the descriptor it
/// assigned (the backend fd). In standalone mode this consults the resolution cache (querying
/// `vfsd` on a miss); a descriptor that is not a socket is rejected with `ENOTSOCK`. The flat
/// descriptor never reaches `networkd`.
#[cfg(feature = "standalone")]
pub(crate) fn resolve_socket(fd: i32, syscall_name: &str) -> Result<i32, Error> {
    use ::sys::error::ErrorCode;

    match resolve(fd) {
        Some(resolution) if resolution.route == Route::Socket => Ok(resolution.backend_fd),
        _ => {
            ::syslog::warn!("{syscall_name}(): not a socket fd={fd}");
            Err(Error::new(ErrorCode::NotSocketFile, "fd is not a socket"))
        },
    }
}

/// Resolves `fd` to the descriptor expected by the socket backend in hosted mode.
///
/// Non-standalone builds route socket syscalls to the host, which interprets the caller-facing
/// descriptor directly, so the descriptor is returned unchanged.
#[cfg(not(feature = "standalone"))]
pub(crate) fn resolve_socket(fd: i32, _syscall_name: &str) -> Result<i32, Error> {
    Ok(fd)
}

/// Resolves `fd` for an operation that `vfsd` serves on its flat slot table itself — the descriptor
/// flag and identity queries (`fcntl(F_GETFD/F_SETFD/F_GETFL/F_SETFL)`, `fstat`) and the descriptor
/// duplication family (`dup`/`dup2`/`fcntl(F_DUPFD)`).
///
/// Unlike [`resolve_vfs`], this accepts a console descriptor as well as a `vfsd`-served one, and it
/// returns the *caller-facing flat descriptor* rather than the resolved `backend_fd`. That
/// distinction matters for the console: its `backend_fd` is the standard stream number used to
/// route I/O to the kernel, but these operations act on the slot `vfsd` owns, which is addressed by
/// the flat number. Sockets and unknown descriptors are rejected (socket participation in this
/// table lands in a later plan).
#[cfg(feature = "standalone")]
pub(crate) fn resolve_table_op(fd: i32, syscall_name: &str) -> Result<i32, Error> {
    use ::sys::error::ErrorCode;

    match resolve(fd) {
        Some(resolution) if matches!(resolution.route, Route::Vfs | Route::Console) => Ok(fd),
        _ => {
            ::syslog::warn!("{syscall_name}(): bad file descriptor fd={fd}");
            Err(Error::new(ErrorCode::BadFile, "fd is not a vfsd table descriptor"))
        },
    }
}

/// Resolves `fd` for a `vfsd` flat-table operation in hosted mode.
///
/// Non-standalone builds route these syscalls to `linuxd`, which interprets the caller-facing
/// descriptor directly, so the descriptor is returned unchanged.
#[cfg(not(feature = "standalone"))]
pub(crate) fn resolve_table_op(fd: i32, _syscall_name: &str) -> Result<i32, Error> {
    Ok(fd)
}

/// Records the resolution learned for `fd` from a backend response, stamped with the `epoch` the
/// backend reported.
///
/// Called when a descriptor is created (`open`/`pipe`/`socket`) or learned from a `vfsd` resolution
/// so the cache holds an authoritative entry rather than re-querying on every use. The reported
/// generation also advances [`EXPECTED_EPOCH`], so an entry recorded now is coherent against the
/// table state that produced it. An existing entry for `fd` is replaced.
#[cfg(any(feature = "standalone", test))]
pub(crate) fn record(fd: i32, route: Route, backend_fd: i32, epoch: u64) {
    observe_epoch(epoch);
    CACHE.lock().insert(
        fd,
        CacheEntry {
            route,
            backend_fd,
            epoch,
        },
    );
}

/// Queries `vfsd` for the authoritative route of `fd` and records the answer.
///
/// Reached only when the cache misses (or holds a stale entry) — i.e. for a `vfsd`-owned descriptor
/// that this process did not itself create, or one whose entry went stale. The answer is recorded
/// so subsequent uses hit the cache. Returns `None` if `vfsd` reports no slot for `fd` (an invalid
/// descriptor).
#[cfg(all(feature = "standalone", not(test)))]
fn resolve_via_vfsd(fd: i32) -> VfsdResolution {
    use crate::{
        unistd::message::{
            ResolveFdRequest,
            ResolveFdResponse,
        },
        SystemCallMessage,
        SystemCallMessageHeader,
    };
    use ::sys::{
        ipc::Message,
        pm::ThreadIdentifier,
    };

    let tid: ThreadIdentifier = match ::sys::kcall::pm::__kcall_gettid() {
        Ok(tid) => tid,
        Err(_) => return VfsdResolution::Unavailable,
    };
    let pid: ::sys::pm::ProcessIdentifier = match ::sys::kcall::pm::getpid() {
        Ok(pid) => pid,
        Err(_) => return VfsdResolution::Unavailable,
    };

    // Send the resolution query to vfsd and await its authoritative answer.
    let request: Message =
        ResolveFdRequest::build(tid, pid, fd, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE);
    if ::sys::kcall::ipc::__kcall_send(&request).is_err() {
        return VfsdResolution::Unavailable;
    }
    let response: Message = match ::sys::kcall::ipc::__kcall_recv() {
        Ok(response) => response,
        Err(_) => return VfsdResolution::Unavailable,
    };

    // A non-zero status means vfsd holds no slot for this descriptor: it is unroutable.
    if response.status != 0 {
        return VfsdResolution::BadFile;
    }

    let message: SystemCallMessage = match SystemCallMessage::try_from_bytes(response.payload) {
        Ok(message) => message,
        Err(_) => return VfsdResolution::BadFile,
    };
    let resolved: ResolveFdResponse = match message.header {
        SystemCallMessageHeader::ResolveFdResponse => {
            ResolveFdResponse::from_bytes(message.payload)
        },
        _ => {
            ::syslog::warn!("resolve_via_vfsd(): unexpected response header (fd={fd})");
            return VfsdResolution::BadFile;
        },
    };
    // `ResolveFdResponse` is `#[repr(C, packed)]`, so read each field through a raw pointer to avoid
    // forming an unaligned reference (undefined behavior on targets that fault on misaligned loads).
    let route_tag: u32 = unsafe { ::core::ptr::addr_of!(resolved.route).read_unaligned() };
    let backend_fd: i32 = unsafe { ::core::ptr::addr_of!(resolved.backend_fd).read_unaligned() };
    let epoch: u64 = unsafe { ::core::ptr::addr_of!(resolved.epoch).read_unaligned() };
    let route: Route = match route_tag {
        ResolveFdResponse::ROUTE_CONSOLE => Route::Console,
        ResolveFdResponse::ROUTE_VFS => Route::Vfs,
        ResolveFdResponse::ROUTE_SOCKET => Route::Socket,
        other => {
            ::syslog::warn!("resolve_via_vfsd(): unknown route tag {other} (fd={fd})");
            return VfsdResolution::BadFile;
        },
    };
    record(fd, route, backend_fd, epoch);
    VfsdResolution::Hit(Resolution { route, backend_fd })
}

/// Test stand-in for the `vfsd` resolution query.
///
/// Host unit tests run without the `standalone` feature, so they cannot perform the real IPC. This
/// reads the descriptor's authoritative route from [`MOCK_VFSD`], which a test populates to model
/// `vfsd`'s table, and records it exactly as the production path would.
#[cfg(test)]
fn resolve_via_vfsd(fd: i32) -> VfsdResolution {
    if MOCK_VFSD_UNAVAILABLE.load(Ordering::Relaxed) {
        return VfsdResolution::Unavailable;
    }
    let Some((route, backend_fd, epoch)) = MOCK_VFSD.lock().get(&fd).copied() else {
        return VfsdResolution::BadFile;
    };
    record(fd, route, backend_fd, epoch);
    VfsdResolution::Hit(Resolution { route, backend_fd })
}

/// Test model of `vfsd`'s authoritative slot table, consulted by the test [`resolve_via_vfsd`].
#[cfg(test)]
static MOCK_VFSD: Mutex<BTreeMap<i32, (Route, i32, u64)>> = Mutex::new(BTreeMap::new());

/// Test switch that models a run mode without a guest `vfsd`.
#[cfg(test)]
static MOCK_VFSD_UNAVAILABLE: AtomicBool = AtomicBool::new(false);

/// Drops any cached resolution for `fd`.
///
/// Called when a descriptor is destroyed (`close`) so a number later reused by a different backend
/// cannot be answered from a stale entry.
#[cfg(any(feature = "standalone", test))]
pub(crate) fn invalidate(fd: i32) {
    CACHE.lock().remove(&fd);
}

/// Drops every cached resolution and resets the coherence and mock state.
///
/// Restores the process-global statics to their initial values so each test starts from a pristine
/// cache, observed generation, and modeled `vfsd` table.
#[cfg(test)]
fn clear() {
    CACHE.lock().clear();
    EXPECTED_EPOCH.store(0, Ordering::Relaxed);
    MOCK_VFSD.lock().clear();
    MOCK_VFSD_UNAVAILABLE.store(false, Ordering::Relaxed);
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes the cache tests: they share the process-global [`CACHE`], [`EXPECTED_EPOCH`], and
    /// [`MOCK_VFSD`], so they must not run concurrently with one another.
    static CACHE_TEST_GUARD: Mutex<()> = Mutex::new(());

    /// Seeds the modeled `vfsd` table for the duration of a test.
    fn mock_vfsd(fd: i32, route: Route, backend_fd: i32, epoch: u64) {
        MOCK_VFSD.lock().insert(fd, (route, backend_fd, epoch));
    }

    /// Tests that a socket descriptor is a flat slot resolved through `vfsd` like any other object:
    /// its number carries no meaning, and `vfsd` reports the `Socket` route together with the
    /// `networkd` descriptor it routes to. A flat number with nothing cached or modeled in `vfsd`
    /// is unroutable.
    #[test]
    fn socket_resolves_via_vfsd() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        // vfsd owns the socket slot and reports the networkd descriptor it routes to.
        mock_vfsd(4, Route::Socket, 2050, 1);
        assert_eq!(
            resolve(4),
            Some(Resolution {
                route: Route::Socket,
                backend_fd: 2050
            }),
            "a socket descriptor must resolve through vfsd's flat table to its networkd descriptor"
        );
        // Flat numbers are not a fixed range; with no cache entry and no vfsd slot they are
        // unroutable rather than silently assumed to be a socket, console, or vfsd file.
        for fd in [0, 1, 2, 5] {
            assert_eq!(resolve(fd), None, "an unknown flat descriptor must be unroutable");
        }

        clear();
    }

    /// Tests that standard descriptors are flat slots whose console route comes from `vfsd`, not
    /// from their number.
    #[test]
    fn standard_descriptor_consults_vfsd() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        mock_vfsd(1, Route::Console, 1, 3);
        assert_eq!(
            resolve(1),
            Some(Resolution {
                route: Route::Console,
                backend_fd: 1
            }),
            "a standard descriptor must resolve through vfsd's flat table"
        );

        clear();
    }

    /// Tests that a present `vfsd` owns standard descriptor validity: an authoritative bad-fd
    /// answer must not fall back to the console by number.
    #[test]
    fn standard_descriptor_bad_file_does_not_fallback() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        assert_eq!(resolve(STDOUT_FILENO), None, "vfsd bad-fd must stay authoritative");

        clear();
    }

    /// Tests the direct-ELF run mode where no guest `vfsd` is available: standard descriptors still
    /// route to the kernel console so stdout/stderr tests can report results.
    #[test]
    fn standard_descriptor_falls_back_when_vfsd_unavailable() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        MOCK_VFSD_UNAVAILABLE.store(true, Ordering::Relaxed);
        assert_eq!(
            resolve(STDOUT_FILENO),
            Some(Resolution {
                route: Route::Console,
                backend_fd: STDOUT_FILENO
            }),
            "stdio must fall back to the kernel console when no vfsd exists"
        );
        assert_eq!(resolve(4), None, "non-stdio flat descriptors do not fall back");

        clear();
    }

    /// Tests the descriptor distinction the console `close` path depends on. A console alias minted
    /// by `fcntl(F_DUPFD)` — e.g. a shell parking stdout at fd 10 while it redirects — resolves to
    /// the console route but reports the *stream number* (`1`) as its `backend_fd`, not its flat
    /// slot (`10`). Because that stream number is also the live console's own flat slot, closing
    /// the alias by `backend_fd` would free the live console and orphan the alias, so `close` must
    /// address vfsd's slot by the caller-facing flat descriptor, never by `backend_fd`.
    #[test]
    fn console_alias_resolves_by_stream_number_not_slot() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        const ALIAS_FD: i32 = 10;
        // The live console stdout: its flat slot and stream number coincide at 1.
        mock_vfsd(STDOUT_FILENO, Route::Console, STDOUT_FILENO, 1);
        // The alias parked at fd 10 by fcntl(F_DUPFD): a distinct slot that still streams to
        // stdout.
        mock_vfsd(ALIAS_FD, Route::Console, STDOUT_FILENO, 1);

        let alias: Resolution = resolve(ALIAS_FD).expect("the alias must resolve");
        assert_eq!(alias.route, Route::Console, "an alias of a console stream is still a console");
        assert_eq!(
            alias.backend_fd, STDOUT_FILENO,
            "a console alias reports its stream number as backend_fd, not its flat slot"
        );
        // The crux of the close fix: backend_fd (the stream number) is the live console's own slot,
        // so it must not be used as the slot key when closing the alias.
        assert_ne!(
            alias.backend_fd, ALIAS_FD,
            "the alias's stream number must differ from its flat slot"
        );
        assert_eq!(
            resolve(STDOUT_FILENO).map(|r| r.backend_fd),
            Some(STDOUT_FILENO),
            "backend_fd collides with the live console's own flat slot"
        );

        clear();
    }

    /// Tests that hosted builds leave VFS descriptor interpretation to linuxd.
    #[test]
    fn hosted_resolve_vfs_returns_raw_fd() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        let backend_fd: i32 = resolve_vfs(7, "test").expect("hosted resolve_vfs should succeed");
        assert_eq!(backend_fd, 7, "hosted mode must pass raw descriptors through");

        clear();
    }

    /// Tests that hosted builds pass a flat-table operation's descriptor through unchanged, leaving
    /// interpretation to linuxd.
    #[test]
    fn hosted_resolve_table_op_returns_raw_fd() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        let backend_fd: i32 =
            resolve_table_op(9, "test").expect("hosted resolve_table_op should succeed");
        assert_eq!(backend_fd, 9, "hosted mode must pass raw descriptors through");

        clear();
    }

    /// Tests that hosted builds leave socket descriptor interpretation to the host backend, passing
    /// the descriptor through unchanged.
    #[test]
    fn hosted_resolve_socket_returns_raw_fd() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        let backend_fd: i32 =
            resolve_socket(11, "test").expect("hosted resolve_socket should succeed");
        assert_eq!(backend_fd, 11, "hosted mode must pass raw descriptors through");

        clear();
    }

    /// Tests that a recorded entry is returned by `resolve` without a `vfsd` round-trip, and that
    /// `backend_fd` may differ from the caller-facing descriptor.
    #[test]
    fn record_then_resolve_hits_cache() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        // Model a contradictory vfsd answer to prove a coherent cache hit never consults it.
        mock_vfsd(4, Route::Socket, 999, 0);
        record(4, Route::Vfs, 41, 0);
        assert_eq!(
            resolve(4),
            Some(Resolution {
                route: Route::Vfs,
                backend_fd: 41
            }),
            "a coherent recorded entry must be returned without querying vfsd"
        );

        clear();
    }

    /// Tests that a cache miss on a flat descriptor consults `vfsd`, returns its authoritative
    /// answer, and caches it so the next use hits the cache.
    #[test]
    fn cache_miss_consults_vfsd() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        mock_vfsd(5, Route::Vfs, 5, 7);
        assert_eq!(
            resolve(5),
            Some(Resolution {
                route: Route::Vfs,
                backend_fd: 5
            }),
            "a cache miss on a flat descriptor must consult vfsd"
        );
        // The answer is now cached at the reported epoch, so removing the vfsd model leaves the hot
        // path intact.
        MOCK_VFSD.lock().clear();
        assert_eq!(
            resolve(5).map(|r| r.route),
            Some(Route::Vfs),
            "the resolved descriptor must now hit the cache"
        );

        clear();
    }

    /// Tests that `invalidate` drops a single entry, after which resolution re-consults `vfsd`.
    #[test]
    fn invalidate_drops_entry() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        mock_vfsd(6, Route::Vfs, 6, 2);
        assert!(resolve(6).is_some(), "the descriptor resolves and is cached");
        invalidate(6);
        // With the entry gone, the next resolve falls through to vfsd again rather than answering
        // from the dropped entry.
        MOCK_VFSD.lock().clear();
        assert_eq!(resolve(6), None, "an invalidated entry must not survive as a stale answer");

        clear();
    }

    /// Tests that `clear` drops every entry.
    #[test]
    fn clear_drops_all() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        record(4, Route::Vfs, 4, 1);
        record(5, Route::Vfs, 5, 1);
        clear();
        assert_eq!(resolve(4), None, "clear must drop recorded entries");
        assert_eq!(resolve(5), None, "clear must drop recorded entries");

        clear();
    }

    /// Tests the stale-route hazard the flat namespace introduces: an entry learned at an older
    /// generation must be refetched once `vfsd`'s table advances, and the fresh backend used — never
    /// the entry that described the number's previous occupant.
    #[test]
    fn stale_entry_is_refetched_from_vfsd() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        // Descriptor 4 is learned as a vfsd file at generation 1.
        record(4, Route::Vfs, 4, 1);
        assert_eq!(resolve(4).map(|r| r.route), Some(Route::Vfs), "initially a vfsd file");

        // A later table mutation (e.g. another open) advances the observed generation, and the
        // number has since been reused for a different backend in vfsd's table.
        record(9, Route::Vfs, 9, 5);
        mock_vfsd(4, Route::Socket, 4, 5);

        // The stale entry must not be used: resolution refetches and reflects the new backend.
        assert_eq!(
            resolve(4),
            Some(Resolution {
                route: Route::Socket,
                backend_fd: 4
            }),
            "a stale entry must be refetched, not trusted"
        );

        clear();
    }

    /// Tests that a coherent entry is never refetched: once the observed generation matches the
    /// entry's epoch, the hot path answers from the cache even if `vfsd` would say otherwise.
    #[test]
    fn coherent_entry_skips_vfsd() {
        let _guard = CACHE_TEST_GUARD.lock();
        clear();

        // Entry and observed generation agree at 5.
        record(4, Route::Vfs, 4, 5);
        // A contradictory vfsd model would change the answer if it were consulted.
        mock_vfsd(4, Route::Socket, 4, 5);
        assert_eq!(
            resolve(4).map(|r| r.route),
            Some(Route::Vfs),
            "a coherent entry must be served from the cache without a vfsd round-trip"
        );

        clear();
    }
}
