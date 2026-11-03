// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::validate_msync_flags;
use ::sys::error::ErrorCode;
use ::sysapi::sys_mman::msync_flags::{
    MS_ASYNC,
    MS_INVALIDATE,
    MS_SYNC,
};

/// A lone `MS_SYNC` flag is accepted.
#[test]
fn accepts_ms_sync() {
    assert_eq!(validate_msync_flags(MS_SYNC), Ok(()));
}

/// A lone `MS_ASYNC` flag is accepted.
#[test]
fn accepts_ms_async() {
    assert_eq!(validate_msync_flags(MS_ASYNC), Ok(()));
}

/// A synchronization mode combined with `MS_INVALIDATE` is accepted.
#[test]
fn accepts_mode_with_invalidate() {
    assert_eq!(validate_msync_flags(MS_SYNC | MS_INVALIDATE), Ok(()));
}

/// Specifying no synchronization mode is rejected.
#[test]
fn rejects_no_mode() {
    assert_eq!(validate_msync_flags(0), Err(ErrorCode::InvalidArgument));
    assert_eq!(validate_msync_flags(MS_INVALIDATE), Err(ErrorCode::InvalidArgument));
}

/// Specifying both synchronization modes is rejected.
#[test]
fn rejects_both_modes() {
    assert_eq!(validate_msync_flags(MS_ASYNC | MS_SYNC), Err(ErrorCode::InvalidArgument));
}

/// An unknown flag bit is rejected.
#[test]
fn rejects_unknown_bit() {
    let unknown: ::sysapi::ffi::c_int = MS_ASYNC | MS_INVALIDATE | MS_SYNC | 0x100;
    assert_eq!(validate_msync_flags(unknown), Err(ErrorCode::InvalidArgument));
}
