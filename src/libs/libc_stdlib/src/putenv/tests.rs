// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::putenv;
use crate::{
    getenv::getenv,
    set_errno,
};
use ::sysapi::{
    errno::EINVAL,
    ffi::{
        c_char,
        c_int,
    },
};

fn get_errno() -> c_int {
    unsafe { *::sysapi::errno::__errno_location() }
}

/// Returns `true` if the C string at `ptr` equals `expected` (excluding the terminator).
unsafe fn cstr_eq(ptr: *const c_char, expected: &[u8]) -> bool {
    if ptr.is_null() {
        return false;
    }
    for (i, &b) in expected.iter().enumerate() {
        if *ptr.add(i) as u8 != b {
            return false;
        }
    }
    *ptr.add(expected.len()) as u8 == 0
}

#[test]
fn sets_variable() {
    // Use a unique name to avoid colliding with other tests that share the
    // process-global environment table.
    let entry = b"NANVIX_PUTENV_SET=hello\0";
    assert_eq!(unsafe { putenv(entry.as_ptr().cast::<c_char>().cast_mut()) }, 0);

    let name = b"NANVIX_PUTENV_SET\0";
    let value = unsafe { getenv(name.as_ptr().cast::<c_char>()) };
    assert!(unsafe { cstr_eq(value, b"hello") });
}

#[test]
fn uses_caller_storage() {
    let mut entry = *b"NANVIX_PUTENV_MUT=before\0";
    assert_eq!(unsafe { putenv(entry.as_mut_ptr().cast::<c_char>()) }, 0);

    let value_start: usize = b"NANVIX_PUTENV_MUT=".len();
    entry[value_start..value_start + b"second".len()].copy_from_slice(b"second");

    let name = b"NANVIX_PUTENV_MUT\0";
    let value = unsafe { getenv(name.as_ptr().cast::<c_char>()) };
    assert!(unsafe { cstr_eq(value, b"second") });

    // `putenv()` installs a pointer to the caller's storage in the process-global environment
    // table. Remove the entry while `entry` is still alive so the table never retains a
    // dangling pointer into this stack frame, which a later (or concurrent) environment lookup
    // would otherwise dereference.
    let remove = b"NANVIX_PUTENV_MUT\0";
    assert_eq!(unsafe { putenv(remove.as_ptr().cast::<c_char>().cast_mut()) }, 0);
}

#[test]
fn removes_variable_without_equals() {
    let entry = b"NANVIX_PUTENV_DEL=value\0";
    assert_eq!(unsafe { putenv(entry.as_ptr().cast::<c_char>().cast_mut()) }, 0);

    // A string without '=' removes the named variable.
    let remove = b"NANVIX_PUTENV_DEL\0";
    assert_eq!(unsafe { putenv(remove.as_ptr().cast::<c_char>().cast_mut()) }, 0);

    let value = unsafe { getenv(remove.as_ptr().cast::<c_char>()) };
    assert!(value.is_null());
}

#[test]
fn null_string_fails() {
    set_errno(0);
    assert_eq!(unsafe { putenv(::core::ptr::null_mut()) }, -1);
    assert_eq!(get_errno(), EINVAL);
}
