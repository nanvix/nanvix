// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::core::sync::atomic::AtomicBool;

/// Test model of `vfsd`'s authoritative slot table, consulted by the test [`resolve_via_vfsd`].
static MOCK_VFSD: Mutex<BTreeMap<i32, (Route, i32, u64)>> = Mutex::new(BTreeMap::new());

/// Test switch that models an unavailable guest `vfsd`.
static MOCK_VFSD_UNAVAILABLE: AtomicBool = AtomicBool::new(false);

/// Test switch that makes the modeled `vfsd` lookup return `EINTR`.
static MOCK_VFSD_INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Test stand-in for the `vfsd` resolution query.
///
/// Host unit tests cannot perform the real IPC. This reads the descriptor's authoritative route
/// from [`MOCK_VFSD`], which a test populates to model `vfsd`'s table, and records it exactly as
/// the production path would.
pub(super) fn resolve_via_vfsd(fd: i32) -> Result<VfsdResolution, Error> {
    if MOCK_VFSD_INTERRUPTED.load(Ordering::Relaxed) {
        return Err(Error::new(
            ::sys::error::ErrorCode::Interrupted,
            "mock descriptor resolution interrupted",
        ));
    }
    if MOCK_VFSD_UNAVAILABLE.load(Ordering::Relaxed) {
        return Ok(VfsdResolution::Unavailable);
    }
    let Some((route, backend_fd, epoch)) = MOCK_VFSD.lock().get(&fd).copied() else {
        return Ok(VfsdResolution::BadFile);
    };
    record(fd, route, backend_fd, epoch);
    Ok(VfsdResolution::Hit(Resolution { route, backend_fd }))
}

/// Resolves a descriptor in unit tests, whose modeled authority cannot fail.
fn resolve(fd: i32) -> Option<Resolution> {
    resolve_result(fd).expect("mock descriptor resolution should succeed")
}

/// Drops every cached resolution and resets the coherence and mock state.
///
/// Restores the process-global statics to their initial values so each test starts from a pristine
/// cache, observed generation, and modeled `vfsd` table.
fn clear() {
    CACHE.lock().clear();
    EXPECTED_EPOCH.store(0, Ordering::Relaxed);
    MOCK_VFSD.lock().clear();
    MOCK_VFSD_UNAVAILABLE.store(false, Ordering::Relaxed);
    MOCK_VFSD_INTERRUPTED.store(false, Ordering::Relaxed);
}

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

/// Tests that a vfsd-served terminal supports both terminal control and VFS data routing.
#[test]
fn terminal_route_supports_control_and_io() {
    let _guard = CACHE_TEST_GUARD.lock();
    clear();

    mock_vfsd(4, Route::Terminal, 4, 1);
    assert_eq!(resolve_tty(4, "ioctl").expect("resolve terminal control"), 4);
    assert_eq!(resolve_vfs(4, "read").expect("resolve terminal I/O"), 4);
    assert!(matches!(resolve_console(4), Ok(ConsoleLookup::Other)));

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

/// Tests the direct-ELF path where no guest `vfsd` is available: standard descriptors still
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

/// Tests that an interrupted authoritative lookup is not converted into a descriptor error.
#[test]
fn interrupted_lookup_is_preserved() {
    let _guard = CACHE_TEST_GUARD.lock();
    clear();

    MOCK_VFSD_INTERRUPTED.store(true, Ordering::Relaxed);
    let error: Error = resolve_result(4).expect_err("resolution should report interruption");
    assert_eq!(error.code, ::sys::error::ErrorCode::Interrupted);

    let error: Error = match resolve_console(STDIN_FILENO) {
        Err(error) => error,
        Ok(_) => panic!("console resolution should report interruption"),
    };
    assert_eq!(error.code, ::sys::error::ErrorCode::Interrupted);

    clear();
}
