// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Environment-Variable Runtime API Tests (single process)
//!
//! Exercises the POSIX `getenv()`, `setenv()`, and `unsetenv()` runtime environment API provided
//! by `libc_stdlib`, within a single process (no `fork()`). The `fork()`-based isolation scenarios
//! are covered by the dedicated `setenv-rust` test.
//!
//! Behavior conforms to the POSIX specification:
//! - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getenv.html>
//! - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/setenv.html>
//! - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/unsetenv.html>

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ffi::CStr;
use ::sys::error::Error;
use ::sysapi::{
    errno::EINVAL,
    ffi::{
        c_char,
        c_int,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Non-zero `overwrite` flag for `setenv()` (replace an existing value).
const OVERWRITE: c_int = 1;

/// Zero `overwrite` flag for `setenv()` (keep an existing value).
const NO_OVERWRITE: c_int = 0;

//==================================================================================================
// Helpers
//==================================================================================================

/// Reads the calling thread's `errno`.
fn read_errno() -> c_int {
    // SAFETY: `__errno_location()` returns a valid pointer to the thread-local `errno`.
    unsafe { *::syscall::errno::__errno_location() }
}

/// Clears the calling thread's `errno` to `0` so a later `EINVAL` check cannot pass on a stale
/// value left behind by a previous test.
fn clear_errno() {
    // SAFETY: `__errno_location()` returns a valid pointer to the thread-local `errno`.
    unsafe { *::syscall::errno::__errno_location() = 0 };
}

/// Returns a `*const c_char` aimed at the given NUL-terminated byte literal.
fn cstr(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr().cast::<c_char>()
}

/// Calls `setenv()` with NUL-terminated `name`/`value` byte literals.
fn do_setenv(name: &[u8], value: &[u8], overwrite: c_int) -> c_int {
    // SAFETY: `name` and `value` are NUL-terminated byte literals that outlive the call.
    unsafe { ::libc_stdlib::setenv(cstr(name), cstr(value), overwrite) }
}

/// Calls `unsetenv()` with a NUL-terminated `name` byte literal.
fn do_unsetenv(name: &[u8]) -> c_int {
    // SAFETY: `name` is a NUL-terminated byte literal that outlives the call.
    unsafe { ::libc_stdlib::unsetenv(cstr(name)) }
}

/// Calls `getenv()` with a NUL-terminated `name` byte literal.
fn do_getenv(name: &[u8]) -> *mut c_char {
    // SAFETY: `name` is a NUL-terminated byte literal that outlives the call.
    unsafe { ::libc_stdlib::getenv(cstr(name)) }
}

/// Asserts that `getenv(name)` returns the value `expected`.
fn assert_getenv_eq(name: &[u8], expected: &[u8]) {
    let ptr: *mut c_char = do_getenv(name);
    assert!(!ptr.is_null(), "getenv() returned null for an existing variable");
    // SAFETY: a non-null `getenv()` result points to a NUL-terminated C string.
    let value: &[u8] = unsafe { CStr::from_ptr(ptr) }.to_bytes();
    assert_eq!(value, expected, "getenv() returned an unexpected value");
}

/// Asserts that `getenv(name)` returns a null pointer.
fn assert_getenv_null(name: &[u8]) {
    let ptr: *mut c_char = do_getenv(name);
    assert!(ptr.is_null(), "getenv() returned a non-null pointer for a missing variable");
}

//==================================================================================================
// Tests
//==================================================================================================

/// Tests that a value set with `setenv()` is retrievable with `getenv()`.
fn test_set_and_get() {
    assert_eq!(do_setenv(b"NANVIX_ENV_RT\0", b"value-rt\0", OVERWRITE), 0, "setenv() failed");
    assert_getenv_eq(b"NANVIX_ENV_RT\0", b"value-rt");
}

/// Tests that `getenv()` returns null for a variable that was never set.
fn test_get_missing() {
    assert_getenv_null(b"NANVIX_ENV_MISSING\0");
}

/// Tests that `getenv(NULL)` returns null rather than dereferencing the pointer.
fn test_get_null_name() {
    // SAFETY: passing a null pointer is exactly what this test exercises.
    let ptr: *mut c_char = unsafe { ::libc_stdlib::getenv(::core::ptr::null()) };
    assert!(ptr.is_null(), "getenv(NULL) must return null");
}

/// Tests that `setenv()` with a non-zero `overwrite` replaces an existing value.
fn test_overwrite_replaces() {
    assert_eq!(do_setenv(b"NANVIX_ENV_OW\0", b"first\0", OVERWRITE), 0, "setenv() failed");
    assert_eq!(do_setenv(b"NANVIX_ENV_OW\0", b"second\0", OVERWRITE), 0, "setenv() failed");
    assert_getenv_eq(b"NANVIX_ENV_OW\0", b"second");
}

/// Tests that `setenv()` with `overwrite == 0` keeps an existing value but still succeeds.
fn test_no_overwrite_keeps() {
    assert_eq!(do_setenv(b"NANVIX_ENV_NOW\0", b"first\0", OVERWRITE), 0, "setenv() failed");
    assert_eq!(
        do_setenv(b"NANVIX_ENV_NOW\0", b"second\0", NO_OVERWRITE),
        0,
        "setenv() with overwrite=0 must succeed even when the variable exists"
    );
    assert_getenv_eq(b"NANVIX_ENV_NOW\0", b"first");
}

/// Tests that `setenv()` with `overwrite == 0` creates a variable that does not yet exist.
fn test_no_overwrite_creates() {
    assert_eq!(do_setenv(b"NANVIX_ENV_NEW\0", b"created\0", NO_OVERWRITE), 0, "setenv() failed");
    assert_getenv_eq(b"NANVIX_ENV_NEW\0", b"created");
}

/// Tests that `unsetenv()` removes a variable.
fn test_unset_removes() {
    assert_eq!(do_setenv(b"NANVIX_ENV_DEL\0", b"x\0", OVERWRITE), 0, "setenv() failed");
    assert_eq!(do_unsetenv(b"NANVIX_ENV_DEL\0"), 0, "unsetenv() failed");
    assert_getenv_null(b"NANVIX_ENV_DEL\0");
}

/// Tests that `unsetenv()` of a variable that does not exist still succeeds (POSIX).
fn test_unset_missing_succeeds() {
    assert_eq!(
        do_unsetenv(b"NANVIX_ENV_NEVER\0"),
        0,
        "unsetenv() of a missing variable must succeed"
    );
}

/// Tests that a value containing `=` is stored and retrieved verbatim.
fn test_value_with_equals() {
    assert_eq!(do_setenv(b"NANVIX_ENV_EQ\0", b"a=b=c\0", OVERWRITE), 0, "setenv() failed");
    assert_getenv_eq(b"NANVIX_ENV_EQ\0", b"a=b=c");
}

/// Tests that the pointer returned by `getenv()` stays valid until the next mutation of that key.
fn test_pointer_stability() {
    assert_eq!(do_setenv(b"NANVIX_ENV_PTR\0", b"stable\0", OVERWRITE), 0, "setenv() failed");
    let ptr: *mut c_char = do_getenv(b"NANVIX_ENV_PTR\0");
    assert!(!ptr.is_null(), "getenv() returned null");

    // Mutating a *different* key must not invalidate the pointer to this key's value.
    assert_eq!(do_setenv(b"NANVIX_ENV_PTR_OTHER\0", b"other\0", OVERWRITE), 0, "setenv() failed");

    // SAFETY: the pointer is still valid because no mutation touched `NANVIX_ENV_PTR`.
    let value: &[u8] = unsafe { CStr::from_ptr(ptr) }.to_bytes();
    assert_eq!(value, b"stable", "getenv() pointer was invalidated by an unrelated setenv()");
}

/// Tests that `setenv(NULL, ...)` fails with `EINVAL`.
fn test_setenv_null_name_einval() {
    clear_errno();
    // SAFETY: passing a null name is exactly what this test exercises.
    let ret: c_int = unsafe { ::libc_stdlib::setenv(::core::ptr::null(), cstr(b"v\0"), OVERWRITE) };
    assert_eq!(ret, -1, "setenv() with a null name must fail");
    assert_eq!(read_errno(), EINVAL, "setenv() with a null name must set EINVAL");
}

/// Tests that `setenv(name, NULL, ...)` fails with `EINVAL`.
fn test_setenv_null_value_einval() {
    clear_errno();
    // SAFETY: passing a null value is exactly what this test exercises.
    let ret: c_int =
        unsafe { ::libc_stdlib::setenv(cstr(b"NANVIX_ENV_NV\0"), ::core::ptr::null(), OVERWRITE) };
    assert_eq!(ret, -1, "setenv() with a null value must fail");
    assert_eq!(read_errno(), EINVAL, "setenv() with a null value must set EINVAL");
}

/// Tests that `setenv()` with an empty name fails with `EINVAL`.
fn test_setenv_empty_name_einval() {
    clear_errno();
    assert_eq!(do_setenv(b"\0", b"v\0", OVERWRITE), -1, "setenv() with an empty name must fail");
    assert_eq!(read_errno(), EINVAL, "setenv() with an empty name must set EINVAL");
}

/// Tests that `setenv()` with a `=` in the name fails with `EINVAL`.
fn test_setenv_equals_name_einval() {
    clear_errno();
    assert_eq!(
        do_setenv(b"BAD=NAME\0", b"v\0", OVERWRITE),
        -1,
        "setenv() with '=' in the name must fail"
    );
    assert_eq!(read_errno(), EINVAL, "setenv() with '=' in the name must set EINVAL");
}

/// Tests that `unsetenv(NULL)` fails with `EINVAL`.
fn test_unsetenv_null_einval() {
    clear_errno();
    // SAFETY: passing a null name is exactly what this test exercises.
    let ret: c_int = unsafe { ::libc_stdlib::unsetenv(::core::ptr::null()) };
    assert_eq!(ret, -1, "unsetenv(NULL) must fail");
    assert_eq!(read_errno(), EINVAL, "unsetenv(NULL) must set EINVAL");
}

/// Tests that `unsetenv()` with an empty name fails with `EINVAL`.
fn test_unsetenv_empty_einval() {
    clear_errno();
    assert_eq!(do_unsetenv(b"\0"), -1, "unsetenv() with an empty name must fail");
    assert_eq!(read_errno(), EINVAL, "unsetenv() with an empty name must set EINVAL");
}

/// Tests that `unsetenv()` with a `=` in the name fails with `EINVAL`.
fn test_unsetenv_equals_einval() {
    clear_errno();
    assert_eq!(do_unsetenv(b"BAD=NAME\0"), -1, "unsetenv() with '=' in the name must fail");
    assert_eq!(read_errno(), EINVAL, "unsetenv() with '=' in the name must set EINVAL");
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// Runs all environment variable tests.
pub fn run() -> Result<(), Error> {
    test_set_and_get();
    test_get_missing();
    test_get_null_name();
    test_overwrite_replaces();
    test_no_overwrite_keeps();
    test_no_overwrite_creates();
    test_unset_removes();
    test_unset_missing_succeeds();
    test_value_with_equals();
    test_pointer_stability();
    test_setenv_null_name_einval();
    test_setenv_null_value_einval();
    test_setenv_empty_name_einval();
    test_setenv_equals_name_einval();
    test_unsetenv_null_einval();
    test_unsetenv_empty_einval();
    test_unsetenv_equals_einval();
    Ok(())
}
