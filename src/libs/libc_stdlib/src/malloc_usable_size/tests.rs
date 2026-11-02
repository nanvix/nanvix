// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::malloc_usable_size;
use ::sysapi::sys_types::c_size_t;

#[test]
fn null_pointer_returns_zero() {
    let sz = unsafe { malloc_usable_size(core::ptr::null_mut()) };
    assert_eq!(sz, 0);
}

#[test]
fn malloc_and_query_size() {
    let req: c_size_t = 128;
    let p = unsafe { crate::malloc(req) };
    assert!(!p.is_null());
    let sz = unsafe { malloc_usable_size(p) };
    assert!(sz as usize >= req as usize);
    assert_eq!(sz, req, "usable size should be exactly requested size");
    unsafe { crate::free(p) };
}

#[test]
fn realloc_grow_and_query() {
    let p = unsafe { crate::malloc(64) };
    assert!(!p.is_null());
    let p2 = unsafe { crate::realloc(p, 200) };
    assert!(!p2.is_null());
    let sz2 = unsafe { malloc_usable_size(p2) };
    assert!(sz2 as usize >= 200);
    assert_eq!(sz2, 200);
    unsafe { crate::free(p2) };
}

#[test]
fn aligned_alloc_and_query() {
    let p = unsafe { crate::aligned_alloc(128, 384) };
    assert!(!p.is_null());
    let sz = unsafe { malloc_usable_size(p) };
    assert!(sz as usize >= 384);
    assert_eq!(sz, 384);
    unsafe { crate::free(p) };
}
