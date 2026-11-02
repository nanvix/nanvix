// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::validate_mlockall_flags;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_mman::mlockall_flags::{
        MCL_CURRENT,
        MCL_FUTURE,
    },
};

/// A lone `MCL_CURRENT` flag is accepted.
#[test]
fn accepts_mcl_current() {
    assert_eq!(validate_mlockall_flags(MCL_CURRENT), Ok(()));
}

/// A lone `MCL_FUTURE` flag is accepted.
#[test]
fn accepts_mcl_future() {
    assert_eq!(validate_mlockall_flags(MCL_FUTURE), Ok(()));
}

/// Both flags combined are accepted.
#[test]
fn accepts_both_flags() {
    assert_eq!(validate_mlockall_flags(MCL_CURRENT | MCL_FUTURE), Ok(()));
}

/// Requesting no kind of locking is rejected.
#[test]
fn rejects_no_flags() {
    assert_eq!(validate_mlockall_flags(0), Err(ErrorCode::InvalidArgument));
}

/// Every bit other than the two flags is rejected, alone or combined with the flags.
#[test]
fn rejects_unknown_bits() {
    let known: c_int = MCL_CURRENT | MCL_FUTURE;
    for bit in 0..c_int::BITS {
        let unknown: c_int = 1 << bit;
        if unknown & known != 0 {
            continue;
        }

        for base in [0, MCL_CURRENT, MCL_FUTURE, known] {
            let flags: c_int = base | unknown;
            assert_eq!(
                validate_mlockall_flags(flags),
                Err(ErrorCode::InvalidArgument),
                "flags={flags:#x}"
            );
        }
    }
}
