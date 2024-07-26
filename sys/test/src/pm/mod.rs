// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::libnanvix::{
    ProcessIdentifier,
    ThreadIdentifier,
    UserIdentifier,
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
                true
            } else {
                libnanvix::log!("expected: {:?}, got: {:?}", expected, pid);
                false
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
                true
            } else {
                libnanvix::log!("expected: {:?}, got: {:?}", expected, tid);
                false
            }
        },
        _ => false,
    }
}

//==================================================================================================
// Tests for getuid()
//==================================================================================================

///
/// # Description
///
/// Tests if [`libnanvix::getuid()`] returns the expected value.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_getuid() -> bool {
    match libnanvix::getuid() {
        Ok(uid) => {
            let expected: UserIdentifier = UserIdentifier::ROOT;
            if uid == expected {
                true
            } else {
                libnanvix::log!("expected: {:?}, got: {:?}", expected, uid);
                false
            }
        },
        _ => false,
    }
}

//==================================================================================================
// Tests for geteuid()
//==================================================================================================

///
/// # Description
///
/// Tests if [`libnanvix::geteuid()`] returns the expected value.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_geteuid() -> bool {
    match libnanvix::geteuid() {
        Ok(euid) => {
            let expected: UserIdentifier = UserIdentifier::ROOT;
            if euid == expected {
                true
            } else {
                libnanvix::log!("expected: {:?}, got: {:?}", expected, euid);
                false
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
    crate::test!(test_getuid());
    crate::test!(test_geteuid());
}
