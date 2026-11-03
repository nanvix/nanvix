// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::set_errno;
use ::sysapi::{
    errno::EINVAL,
    ffi::{
        c_int,
        c_void,
    },
    sys_types::c_size_t,
};

// Helper to read errno safely.
fn get_errno() -> c_int {
    unsafe { *sysapi::errno::__errno_location() }
}

#[test]
fn null_storage() {
    set_errno(0);
    let r = unsafe { super::posix_memalign(core::ptr::null_mut(), 64, 128) };
    assert_eq!(r, EINVAL);
    assert_eq!(get_errno(), 0);
}

#[test]
fn zero_size_allocation() {
    set_errno(0);
    let mut out: *mut c_void = core::ptr::null_mut();
    let r = unsafe { super::posix_memalign(&mut out as *mut *mut c_void, 64, 0) };
    assert_eq!(r, EINVAL);
    assert_eq!(get_errno(), 0);
}

#[test]
fn zero_size_alignment() {
    set_errno(0);
    let mut out: *mut c_void = core::ptr::null_mut();
    let r = unsafe { super::posix_memalign(&mut out as *mut *mut c_void, 0, 128) };
    assert_eq!(r, EINVAL);
    assert_eq!(get_errno(), 0);
}

#[test]
fn non_power_of_two_alignment() {
    set_errno(0);
    let mut out: *mut c_void = core::ptr::null_mut();
    let r = unsafe { super::posix_memalign(&mut out as *mut *mut c_void, 24, 128) };
    assert_eq!(r, EINVAL);
    assert_eq!(get_errno(), 0);
}

#[test]
fn alignment_not_multiple_of_pointer_size() {
    set_errno(0);
    let mut out: *mut c_void = core::ptr::null_mut();
    let r = unsafe { super::posix_memalign(&mut out as *mut *mut c_void, 20, 128) };
    assert_eq!(r, EINVAL);
    assert_eq!(get_errno(), 0);
}

#[test]
fn valid_allocation() {
    let mut out: *mut c_void = core::ptr::null_mut();
    let alignment: c_size_t = 128;
    let size: c_size_t = 512;
    let ret = unsafe { super::posix_memalign(&mut out as *mut *mut c_void, alignment, size) };
    assert_eq!(ret, 0);
    assert!(!out.is_null());
    let addr = out as usize;
    assert_eq!(addr & (alignment as usize - 1), 0);
    unsafe {
        crate::free(out);
    }
}
