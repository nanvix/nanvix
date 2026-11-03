// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::core::ffi;
use ::sysapi::ffi::c_char;

/// Tests that `setenv()` accepts a non-UTF-8 value and that it can be retrieved intact.
#[test]
fn test_setenv_non_utf8_value() {
    let name: &[u8] = b"FFI_BIN_KEY\0";
    let value: &[u8] = b"\xff\xfe\xfd\0";
    let ret: c_int =
        unsafe { setenv(name.as_ptr().cast::<c_char>(), value.as_ptr().cast::<c_char>(), 1) };
    assert_eq!(ret, 0);
    let ptr: *const c_char = env_table::get("FFI_BIN_KEY");
    assert!(!ptr.is_null());
    let retrieved: &ffi::CStr = unsafe { ffi::CStr::from_ptr(ptr) };
    assert_eq!(retrieved.to_bytes(), b"\xff\xfe\xfd");
}
