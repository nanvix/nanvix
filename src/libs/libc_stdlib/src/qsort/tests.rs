// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    qsort,
    qsort_r,
};
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
fn sort_integers() {
    let mut arr: [c_int; 5] = [5, 3, 1, 4, 2];
    unsafe {
        qsort(arr.as_mut_ptr().cast::<c_void>(), 5, 4, Some(compare_ints));
    }
    assert_eq!(arr, [1, 2, 3, 4, 5]);
}

#[test]
fn empty_array() {
    let mut arr: [c_int; 0] = [];
    unsafe {
        qsort(arr.as_mut_ptr().cast::<c_void>(), 0, 4, Some(compare_ints));
    }
    assert_eq!(arr, []);
}

#[test]
fn single_element() {
    let mut arr: [c_int; 1] = [42];
    unsafe {
        qsort(arr.as_mut_ptr().cast::<c_void>(), 1, 4, Some(compare_ints));
    }
    assert_eq!(arr, [42]);
}

#[test]
fn already_sorted() {
    let mut arr: [c_int; 4] = [1, 2, 3, 4];
    unsafe {
        qsort(arr.as_mut_ptr().cast::<c_void>(), 4, 4, Some(compare_ints));
    }
    assert_eq!(arr, [1, 2, 3, 4]);
}

unsafe extern "C" fn compare_ints_with_direction(
    a: *const c_void,
    b: *const c_void,
    arg: *mut c_void,
) -> c_int {
    let direction: c_int = *arg.cast::<c_int>();
    compare_ints(a, b) * direction
}

#[test]
fn sort_with_context() {
    let mut arr: [c_int; 5] = [5, 3, 1, 4, 2];
    let mut direction: c_int = -1;
    unsafe {
        qsort_r(
            arr.as_mut_ptr().cast::<c_void>(),
            5,
            4,
            Some(compare_ints_with_direction),
            (&mut direction as *mut c_int).cast::<c_void>(),
        );
    }
    assert_eq!(arr, [5, 4, 3, 2, 1]);
}
