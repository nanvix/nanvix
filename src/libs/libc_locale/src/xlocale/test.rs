// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::sync::{
    Mutex,
    MutexGuard,
};

/// Serializes tests that observe the process-global current locale. They mutate and query the
/// shared `CURRENT_LOCALE`, so they must not run concurrently with one another.
static USELOCALE_GUARD: Mutex<()> = Mutex::new(());

/// Acquires the serialization guard, recovering from poisoning so that a failing test does not
/// cascade into unrelated ones.
fn guard() -> MutexGuard<'static, ()> {
    USELOCALE_GUARD.lock().unwrap_or_else(|e| e.into_inner())
}

#[test]
fn test_newlocale_returns_non_null() {
    let handle: *mut c_void = newlocale(0, ::core::ptr::null(), ::core::ptr::null_mut());
    assert!(!handle.is_null());
}

#[test]
fn test_duplocale_returns_non_null() {
    let handle: *mut c_void = duplocale(::core::ptr::null_mut());
    assert!(!handle.is_null());
}

#[test]
fn test_uselocale_query_is_pure() {
    let _guard: MutexGuard<'static, ()> = guard();

    // Querying must not change the current locale and, initially, reports the global locale.
    let first: *mut c_void = uselocale(::core::ptr::null_mut());
    let second: *mut c_void = uselocale(::core::ptr::null_mut());
    assert_eq!(first, second);
    assert_eq!(first, lc_global_locale());
}

#[test]
fn test_uselocale_round_trips_a_handle() {
    let _guard: MutexGuard<'static, ()> = guard();

    let handle: *mut c_void = newlocale(0, ::core::ptr::null(), ::core::ptr::null_mut());

    // Installing a handle returns the previous (global) locale.
    let previous: *mut c_void = uselocale(handle);
    assert_eq!(previous, lc_global_locale());

    // The installed handle is now reported by a query.
    assert_eq!(uselocale(::core::ptr::null_mut()), handle);

    // Restoring the global locale returns the handle we installed.
    let restored: *mut c_void = uselocale(lc_global_locale());
    assert_eq!(restored, handle);
    assert_eq!(uselocale(::core::ptr::null_mut()), lc_global_locale());
}
