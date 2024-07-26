// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::libnanvix::{
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
// Tests for getgid()
//==================================================================================================

///
/// # Description
///
/// Tests if [`libnanvix::getgid()`] returns the expected value.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_getgid() -> bool {
    match libnanvix::getgid() {
        Ok(gid) => {
            let expected: GroupIdentifier = GroupIdentifier::ROOT;
            if gid == expected {
                true
            } else {
                libnanvix::log!("expected: {:?}, got: {:?}", expected, gid);
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
/// Tests if [`libnanvix::getegid()`] returns the expected value.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_getegid() -> bool {
    match libnanvix::getegid() {
        Ok(egid) => {
            let expected: GroupIdentifier = GroupIdentifier::ROOT;
            if egid == expected {
                true
            } else {
                libnanvix::log!("expected: {:?}, got: {:?}", expected, egid);
                false
            }
        },
        _ => false,
    }
}

//==================================================================================================
// Tests for capctl()
//==================================================================================================

///
/// # Description
///
/// Tests if [`libnanvix::Capability::ExceptionControl`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_exception_control() -> bool {
    // Attempt to acquire and release exception control capability.
    match libnanvix::capctl(libnanvix::Capability::ExceptionControl, true) {
        Ok(()) => match libnanvix::capctl(libnanvix::Capability::ExceptionControl, false) {
            Ok(()) => true,
            _ => false,
        },
        _ => false,
    }
}

///
/// # Description
///
/// Tests if [`libnanvix::Capability::InterruptControl`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_interrupt_control() -> bool {
    // Attempt to acquire and release interrupt control capability.
    match libnanvix::capctl(libnanvix::Capability::InterruptControl, true) {
        Ok(()) => match libnanvix::capctl(libnanvix::Capability::InterruptControl, false) {
            Ok(()) => true,
            _ => false,
        },
        _ => false,
    }
}

///
/// # Description
///
/// Tests if [`libnanvix::Capability::IoManagement`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_io_management() -> bool {
    // Attempt to acquire and release I/O management capability.
    match libnanvix::capctl(libnanvix::Capability::IoManagement, true) {
        Ok(()) => match libnanvix::capctl(libnanvix::Capability::IoManagement, false) {
            Ok(()) => true,
            _ => false,
        },
        _ => false,
    }
}

///
/// # Description
///
/// Tests if [`libnanvix::Capability::MemoryManagement`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_memory_management() -> bool {
    // Attempt to acquire and release memory management capability.
    match libnanvix::capctl(libnanvix::Capability::MemoryManagement, true) {
        Ok(()) => match libnanvix::capctl(libnanvix::Capability::MemoryManagement, false) {
            Ok(()) => true,
            _ => false,
        },
        _ => false,
    }
}

///
/// # Description
///
/// Tests if [`libnanvix::Capability::ProcessManagement`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_process_management() -> bool {
    // Attempt to acquire and release process management capability.
    match libnanvix::capctl(libnanvix::Capability::ProcessManagement, true) {
        Ok(()) => match libnanvix::capctl(libnanvix::Capability::ProcessManagement, false) {
            Ok(()) => true,
            _ => false,
        },
        _ => false,
    }
}

///
/// # Description
///
/// Attempts to acquire the same capability twice.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_invalid_acquire() -> bool {
    // Attempt to acquire exception control capability twice.
    match libnanvix::capctl(libnanvix::Capability::ExceptionControl, true) {
        Ok(()) => match libnanvix::capctl(libnanvix::Capability::ExceptionControl, true) {
            Ok(()) => return false,
            _ => match libnanvix::capctl(libnanvix::Capability::ExceptionControl, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire interrupt control capability twice.
    match libnanvix::capctl(libnanvix::Capability::InterruptControl, true) {
        Ok(()) => match libnanvix::capctl(libnanvix::Capability::InterruptControl, true) {
            Ok(()) => return false,
            _ => match libnanvix::capctl(libnanvix::Capability::InterruptControl, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire I/O management capability twice.
    match libnanvix::capctl(libnanvix::Capability::IoManagement, true) {
        Ok(()) => match libnanvix::capctl(libnanvix::Capability::IoManagement, true) {
            Ok(()) => return false,
            _ => match libnanvix::capctl(libnanvix::Capability::IoManagement, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire memory management capability twice.
    match libnanvix::capctl(libnanvix::Capability::MemoryManagement, true) {
        Ok(()) => match libnanvix::capctl(libnanvix::Capability::MemoryManagement, true) {
            Ok(()) => return false,
            _ => match libnanvix::capctl(libnanvix::Capability::MemoryManagement, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire process management capability twice.
    match libnanvix::capctl(libnanvix::Capability::ProcessManagement, true) {
        Ok(()) => match libnanvix::capctl(libnanvix::Capability::ProcessManagement, true) {
            Ok(()) => return false,
            _ => match libnanvix::capctl(libnanvix::Capability::ProcessManagement, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    true
}

///
/// # Description
///
/// Attempts to release a capability that was not acquired.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_invalid_release() -> bool {
    // Attempt to release exception control capability without acquiring it.
    match libnanvix::capctl(libnanvix::Capability::ExceptionControl, false) {
        Ok(()) => return false,
        _ => (),
    }

    // Attempt to release interrupt control capability without acquiring it.
    match libnanvix::capctl(libnanvix::Capability::InterruptControl, false) {
        Ok(()) => return false,
        _ => (),
    }

    // Attempt to release I/O management capability without acquiring it.
    match libnanvix::capctl(libnanvix::Capability::IoManagement, false) {
        Ok(()) => return false,
        _ => (),
    }

    // Attempt to release memory management capability without acquiring it.
    match libnanvix::capctl(libnanvix::Capability::MemoryManagement, false) {
        Ok(()) => return false,
        _ => (),
    }

    // Attempt to release process management capability without acquiring it.
    match libnanvix::capctl(libnanvix::Capability::ProcessManagement, false) {
        Ok(()) => return false,
        _ => (),
    }

    true
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
    crate::test!(test_getgid());
    crate::test!(test_getegid());
    crate::test!(test_capctl_exception_control());
    crate::test!(test_capctl_interrupt_control());
    crate::test!(test_capctl_io_management());
    crate::test!(test_capctl_memory_management());
    crate::test!(test_capctl_process_management());
    crate::test!(test_capctl_invalid_acquire());
    crate::test!(test_capctl_invalid_release());
}
