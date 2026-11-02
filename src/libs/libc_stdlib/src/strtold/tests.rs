// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strtold;
use ::sysapi::ffi::c_char;

#[test]
fn delegates_to_strtod() {
    let s = b"0x1.8p+1\0";
    let value: f64 = unsafe { strtold(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert_eq!(value, 3.0);
}
