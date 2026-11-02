// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::store_time;
use ::sysapi::sys_types::time_t;

#[test]
fn test_store_time_writes_when_non_null() {
    // A non-null `tloc` receives a copy of the returned value.
    let mut out: time_t = 0;
    let ret: time_t = unsafe { store_time(12_345 as time_t, &mut out) };
    assert_eq!(ret, 12_345);
    assert_eq!(out, 12_345);
}

#[test]
fn test_store_time_ignores_null() {
    // A null `tloc` is tolerated and the value is still returned.
    let ret: time_t = unsafe { store_time(67_890 as time_t, core::ptr::null_mut()) };
    assert_eq!(ret, 67_890);
}
