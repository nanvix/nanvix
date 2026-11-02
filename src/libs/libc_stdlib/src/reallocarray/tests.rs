// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::reallocarray;
use crate::set_errno;
use ::sysapi::{
    errno::ENOMEM,
    ffi::{
        c_int,
        c_void,
    },
    sys_types::c_size_t,
};

fn get_errno() -> c_int {
    unsafe { *::sysapi::errno::__errno_location() }
}

#[test]
fn reallocarray_allocates_product_size() {
    let ptr: *mut c_void = unsafe { reallocarray(core::ptr::null_mut(), 4, 16) };
    assert!(!ptr.is_null());
    assert_eq!(unsafe { crate::malloc_usable_size(ptr) }, 64);
    unsafe { crate::free(ptr) };
}

#[test]
fn reallocarray_overflow_fails() {
    set_errno(0);
    let ptr: *mut c_void = unsafe { reallocarray(core::ptr::null_mut(), c_size_t::MAX, 2) };
    assert!(ptr.is_null());
    assert_eq!(get_errno(), ENOMEM);
}
