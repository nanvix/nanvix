// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::libnanvix::{
    ProcessIdentifier,
    ThreadIdentifier,
};

//==================================================================================================
// Tests for getpid()
//==================================================================================================

///
/// # Description
///
/// Tests if [`libnanvix::getpid()`] returns the expected value.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
/// # Notes
///
/// - This test assumes that the process identifier of the current process is 2. If the system
///   image changes, this test must be updated.
///
fn test_getpid() -> bool {
    match libnanvix::getpid() {
        Ok(pid) => {
            let expected: ProcessIdentifier = ProcessIdentifier::from(2);
            if pid == expected {
                return true;
            } else {
                libnanvix::log!("expected: {:?}, got: {:?}", expected, pid);
                return false;
            }
        },
        _ => false,
    }
}

//==================================================================================================
// Tests for gettid()
//==================================================================================================

///
/// # Description
///
/// Tests if [`libnanvix::gettid()`] returns the expected value.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
/// # Notes
///
/// - This test assumes that the thread identifier of the current thread is 2. If the system
///   image changes, this test must be updated.
///
fn test_gettid() -> bool {
    match libnanvix::gettid() {
        Ok(tid) => {
            let expected: ThreadIdentifier = ThreadIdentifier::from(2);
            if tid == expected {
                return true;
            } else {
                libnanvix::log!("expected: {:?}, got: {:?}", expected, tid);
                return false;
            }
        },
        _ => false,
    }
}

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests kernel calls in the process management facility.
///
pub fn test() {
    crate::test!(test_getpid());
    crate::test!(test_gettid());
}
