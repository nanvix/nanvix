// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::setlocale;
use ::std::sync::{
    Mutex,
    MutexGuard,
};
use ::sysapi::ffi::c_char;

/// Serializes tests that observe the process-global current locale. They mutate and query the
/// shared `CURRENT_LOCALE`, so they must not run concurrently with one another.
static SETLOCALE_GUARD: Mutex<()> = Mutex::new(());

/// Acquires the serialization guard, recovering from poisoning so that a failing test does not
/// cascade into unrelated ones.
fn guard() -> MutexGuard<'static, ()> {
    SETLOCALE_GUARD.lock().unwrap_or_else(|e| e.into_inner())
}

#[test]
fn test_setlocale_null_query() {
    let _guard: MutexGuard<'static, ()> = guard();
    // After selecting "C", a NULL query must report "C".
    let c_str: [u8; 2] = [b'C', 0];
    unsafe { setlocale(0, c_str.as_ptr().cast::<c_char>()) };
    let ret: *mut c_char = unsafe { setlocale(0, core::ptr::null()) };
    assert!(!ret.is_null());
    // Should point to "C".
    assert_eq!(unsafe { *ret }, c_char::try_from(b'C').expect("should fit in c_char"));
    assert_eq!(unsafe { *ret.add(1) }, 0);
}

#[test]
fn test_setlocale_empty_string() {
    let _guard: MutexGuard<'static, ()> = guard();
    let empty: [u8; 1] = [0];
    let ret: *mut c_char = unsafe { setlocale(0, empty.as_ptr().cast::<c_char>()) };
    assert!(!ret.is_null());
    assert_eq!(unsafe { *ret }, c_char::try_from(b'C').expect("should fit in c_char"));
}

#[test]
fn test_setlocale_c_locale() {
    let _guard: MutexGuard<'static, ()> = guard();
    let c_str: [u8; 2] = [b'C', 0];
    let ret: *mut c_char = unsafe { setlocale(0, c_str.as_ptr().cast::<c_char>()) };
    assert!(!ret.is_null());
    assert_eq!(unsafe { *ret }, c_char::try_from(b'C').expect("should fit in c_char"));
}

#[test]
fn test_setlocale_posix_locale() {
    let _guard: MutexGuard<'static, ()> = guard();
    let posix: [u8; 6] = [b'P', b'O', b'S', b'I', b'X', 0];
    let ret: *mut c_char = unsafe { setlocale(0, posix.as_ptr().cast::<c_char>()) };
    assert!(!ret.is_null());
    assert_eq!(unsafe { *ret }, c_char::try_from(b'P').expect("should fit in c_char"));
}

#[test]
fn test_setlocale_query_reflects_selection() {
    let _guard: MutexGuard<'static, ()> = guard();
    // Selecting "POSIX" must be observable through a subsequent NULL query.
    let posix: [u8; 6] = [b'P', b'O', b'S', b'I', b'X', 0];
    unsafe { setlocale(0, posix.as_ptr().cast::<c_char>()) };
    let ret: *mut c_char = unsafe { setlocale(0, core::ptr::null()) };
    assert!(!ret.is_null());
    assert_eq!(unsafe { *ret }, c_char::try_from(b'P').expect("should fit in c_char"));

    // Switching back to "C" must likewise be observable.
    let c_str: [u8; 2] = [b'C', 0];
    unsafe { setlocale(0, c_str.as_ptr().cast::<c_char>()) };
    let ret: *mut c_char = unsafe { setlocale(0, core::ptr::null()) };
    assert!(!ret.is_null());
    assert_eq!(unsafe { *ret }, c_char::try_from(b'C').expect("should fit in c_char"));
}

#[test]
fn test_setlocale_unsupported() {
    let _guard: MutexGuard<'static, ()> = guard();
    let utf8: [u8; 6] = [b'e', b'n', b'_', b'U', b'S', 0];
    let ret: *mut c_char = unsafe { setlocale(0, utf8.as_ptr().cast::<c_char>()) };
    assert!(ret.is_null());
}
