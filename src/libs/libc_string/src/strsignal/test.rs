// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strsignal;
use ::core::ffi::CStr;
use ::sysapi::ffi::c_char;

fn describe(sig: i32) -> ::std::string::String {
    let ptr: *mut c_char = strsignal(sig);
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

#[test]
fn test_known_signal() {
    assert_eq!(describe(9), "Killed");
    assert_eq!(describe(15), "Terminated");
}

#[test]
fn test_unknown_signal() {
    assert_eq!(describe(12345), "Unknown signal");
}
