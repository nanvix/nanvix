// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::system;
use ::sysapi::ffi::c_char;

#[test]
fn null_command_reports_processor() {
    assert_ne!(unsafe { system(::core::ptr::null()) }, 0);
}

#[test]
fn non_null_command_returns_error() {
    let command = b"true\0";
    assert_eq!(unsafe { system(command.as_ptr().cast::<c_char>()) }, -1);
}
