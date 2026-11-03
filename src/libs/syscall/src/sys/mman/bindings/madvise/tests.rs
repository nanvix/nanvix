// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::validate_madvise_advice;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_mman::memory_advice::{
        MADV_DONTNEED,
        MADV_FREE,
        MADV_NORMAL,
        MADV_RANDOM,
        MADV_SEQUENTIAL,
        MADV_WILLNEED,
    },
};

/// Every conventional advice value is accepted.
#[test]
fn accepts_conventional_advice() {
    for advice in [
        MADV_NORMAL,
        MADV_RANDOM,
        MADV_SEQUENTIAL,
        MADV_WILLNEED,
        MADV_DONTNEED,
        MADV_FREE,
    ] {
        assert_eq!(validate_madvise_advice(advice), Ok(()), "advice={advice}");
    }
}

/// The unassigned values between `MADV_DONTNEED` and `MADV_FREE` are rejected.
#[test]
fn rejects_unassigned_advice() {
    for advice in (MADV_DONTNEED + 1)..MADV_FREE {
        assert_eq!(
            validate_madvise_advice(advice),
            Err(ErrorCode::InvalidArgument),
            "advice={advice}"
        );
    }
}

/// Values outside the range of conventional advice values are rejected.
#[test]
fn rejects_out_of_range_advice() {
    for advice in [c_int::MIN, -1, MADV_FREE + 1, c_int::MAX] {
        assert_eq!(
            validate_madvise_advice(advice),
            Err(ErrorCode::InvalidArgument),
            "advice={advice}"
        );
    }
}
