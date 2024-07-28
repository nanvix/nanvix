// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::pm::{
    GroupIdentifier,
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
/// Tests if [`nvx::getpid()`] returns the expected value.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
/// # Notes
///
/// - This test assumes that the process identifier of the current process is 1. If the system
///   image changes, this test must be updated.
///
fn test_getpid() -> bool {
    match nvx::pm::getpid() {
        Ok(pid) => {
            let expected: ProcessIdentifier = ProcessIdentifier::from(1);
            if pid == expected {
                true
            } else {
                nvx::log!("expected: {:?}, got: {:?}", expected, pid);
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
/// Tests if [`nvx::gettid()`] returns the expected value.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
/// # Notes
///
/// - This test assumes that the thread identifier of the current thread is 1. If the system
///   image changes, this test must be updated.
///
fn test_gettid() -> bool {
    match nvx::pm::gettid() {
        Ok(tid) => {
            let expected: ThreadIdentifier = ThreadIdentifier::from(1);
            if tid == expected {
                true
            } else {
                nvx::log!("expected: {:?}, got: {:?}", expected, tid);
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
/// Tests if [`nvx::getuid()`] returns the expected value.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_getuid() -> bool {
    match nvx::pm::getuid() {
        Ok(uid) => {
            let expected: UserIdentifier = UserIdentifier::ROOT;
            if uid == expected {
                true
            } else {
                nvx::log!("expected: {:?}, got: {:?}", expected, uid);
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
/// Tests if [`nvx::pm::geteuid()`] returns the expected value.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_geteuid() -> bool {
    match nvx::pm::geteuid() {
        Ok(euid) => {
            let expected: UserIdentifier = UserIdentifier::ROOT;
            if euid == expected {
                true
            } else {
                nvx::log!("expected: {:?}, got: {:?}", expected, euid);
                false
            }
        },
        _ => false,
    }
}

//==================================================================================================
// Tests for getgid()
//==================================================================================================

///
/// # Description
///
/// Tests if [`nvx::getgid()`] returns the expected value.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_getgid() -> bool {
    match nvx::pm::getgid() {
        Ok(gid) => {
            let expected: GroupIdentifier = GroupIdentifier::ROOT;
            if gid == expected {
                true
            } else {
                nvx::log!("expected: {:?}, got: {:?}", expected, gid);
                false
            }
        },
        _ => false,
    }
}

//==================================================================================================
// Tests for getegid()
//==================================================================================================

///
/// # Description
///
/// Tests if [`nvx::getegid()`] returns the expected value.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_getegid() -> bool {
    match nvx::pm::getegid() {
        Ok(egid) => {
            let expected: GroupIdentifier = GroupIdentifier::ROOT;
            if egid == expected {
                true
            } else {
                nvx::log!("expected: {:?}, got: {:?}", expected, egid);
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
/// Tests kernel calls for identity management.
///
pub fn test() {
    crate::test!(test_getpid());
    crate::test!(test_gettid());
    crate::test!(test_getuid());
    crate::test!(test_geteuid());
    crate::test!(test_getgid());
    crate::test!(test_getegid());
}
