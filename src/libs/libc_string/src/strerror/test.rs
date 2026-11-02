// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strerror;
use ::std::vec::Vec;
use ::sysapi::{
    errno::{
        EINVAL,
        ENOSYS,
        ENOTSUP,
    },
    ffi::c_char,
};

fn c_str_to_bytes(p: *const c_char) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    let mut i: usize = 0;
    unsafe {
        while *p.add(i) != 0 {
            v.push(u8::from_ne_bytes((*p.add(i)).to_ne_bytes()));
            i += 1;
        }
    }
    v
}

#[test]
fn test_strerror_known() {
    let ret: *mut c_char = unsafe { strerror(EINVAL) };
    assert!(!ret.is_null());
    assert_eq!(c_str_to_bytes(ret), b"Invalid argument");
}

#[test]
fn test_strerror_zero() {
    let ret: *mut c_char = unsafe { strerror(0) };
    assert!(!ret.is_null());
    assert_eq!(c_str_to_bytes(ret), b"Success");
}

#[test]
fn test_strerror_enosys() {
    // ENOSYS is 88 on Nanvix (see sysapi::errno), not 38 as on Linux.
    let ret: *mut c_char = unsafe { strerror(ENOSYS) };
    assert!(!ret.is_null());
    assert_eq!(c_str_to_bytes(ret), b"Function not implemented");
}

#[test]
fn test_strerror_enotsup() {
    // ENOTSUP is 134 on Nanvix, distinct from EOPNOTSUPP (95). The VFS
    // reports ErrorCode::OperationNotSupported (== ENOTSUP) for unsupported
    // operations such as symlink() in standalone mode.
    let ret: *mut c_char = unsafe { strerror(ENOTSUP) };
    assert!(!ret.is_null());
    assert_eq!(c_str_to_bytes(ret), b"Operation not supported");
}

#[test]
fn test_strerror_unknown() {
    let ret: *mut c_char = unsafe { strerror(999) };
    assert!(!ret.is_null());
    assert_eq!(c_str_to_bytes(ret), b"Unknown error");
}
