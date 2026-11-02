// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::secure_getenv;
use ::sysapi::ffi::c_char;

#[test]
fn missing_variable_returns_null() {
    let name = b"NANVIX_SECURE_GETENV_MISSING\0";
    let ptr: *mut c_char = unsafe { secure_getenv(name.as_ptr().cast::<c_char>()) };
    assert!(ptr.is_null());
}
