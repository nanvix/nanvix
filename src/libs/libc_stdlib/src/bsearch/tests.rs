// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::bsearch;
use ::sysapi::ffi::{
    c_int,
    c_void,
};

unsafe extern "C" fn compare_ints(a: *const c_void, b: *const c_void) -> c_int {
    let a_val: c_int = *a.cast::<c_int>();
    let b_val: c_int = *b.cast::<c_int>();
    if a_val < b_val {
        -1
    } else if a_val > b_val {
        1
    } else {
        0
    }
}

#[test]
fn found() {
    let arr: [c_int; 5] = [1, 2, 3, 4, 5];
    let key: c_int = 3;
    let result = unsafe {
        bsearch(
            (&key as *const c_int).cast::<c_void>(),
            arr.as_ptr().cast::<c_void>(),
            5,
            4,
            Some(compare_ints),
        )
    };
    assert!(!result.is_null());
    assert_eq!(unsafe { *result.cast::<c_int>() }, 3);
}

#[test]
fn not_found() {
    let arr: [c_int; 5] = [1, 2, 3, 4, 5];
    let key: c_int = 6;
    let result = unsafe {
        bsearch(
            (&key as *const c_int).cast::<c_void>(),
            arr.as_ptr().cast::<c_void>(),
            5,
            4,
            Some(compare_ints),
        )
    };
    assert!(result.is_null());
}

#[test]
fn empty_array() {
    let arr: [c_int; 0] = [];
    let key: c_int = 1;
    let result = unsafe {
        bsearch(
            (&key as *const c_int).cast::<c_void>(),
            arr.as_ptr().cast::<c_void>(),
            0,
            4,
            Some(compare_ints),
        )
    };
    assert!(result.is_null());
}
