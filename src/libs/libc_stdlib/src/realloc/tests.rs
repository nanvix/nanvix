// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::realloc;
use crate::set_errno;
use ::sysapi::{
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
fn realloc_null_behaves_like_malloc() {
    set_errno(0);
    let size: c_size_t = 128;
    let p = unsafe { realloc(core::ptr::null_mut(), size) };
    assert!(!p.is_null());
    assert_eq!(get_errno(), 0);
    unsafe {
        crate::free(p);
    }
}

#[test]
fn realloc_zero_behaves_like_free() {
    let size: c_size_t = 128;
    let p = unsafe { realloc(core::ptr::null_mut(), size) };
    assert!(!p.is_null());
    set_errno(0);
    let p2 = unsafe { realloc(p, 0) };
    assert!(p2.is_null());
    assert_eq!(get_errno(), 0);
}

#[test]
fn realloc_grow_and_shrink() {
    // Allocate initial block.
    let initial: c_size_t = 32;
    let p = unsafe { realloc(core::ptr::null_mut(), initial) }.cast::<u8>();
    assert!(!p.is_null());
    unsafe {
        for i in 0..initial as usize {
            p.add(i).write(u8::try_from(i).unwrap_or_default());
        }
    }
    // Grow block.
    let grown: c_size_t = 64;
    let p2 = unsafe { realloc(p.cast::<c_void>(), grown) }.cast::<u8>();
    assert!(!p2.is_null());
    unsafe {
        for i in 0..initial as usize {
            assert_eq!(
                p2.add(i).read(),
                u8::try_from(i).unwrap_or_default(),
                "realloc data mismatch at {i} after grow"
            );
        }
        for i in initial as usize..grown as usize {
            p2.add(i).write(u8::try_from(i).unwrap_or_default());
        }
    }
    // Shrink block.
    let shrunk: c_size_t = 16;
    let p3 = unsafe { realloc(p2.cast::<c_void>(), shrunk) }.cast::<u8>();
    assert!(!p3.is_null());
    unsafe {
        for i in 0..shrunk as usize {
            assert_eq!(
                p3.add(i).read(),
                u8::try_from(i).unwrap_or_default(),
                "realloc data mismatch at {i} after shrink"
            );
        }
    }
    unsafe {
        crate::free(p3.cast::<c_void>());
    }
}
