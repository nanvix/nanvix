// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::pm::Capability;

//==================================================================================================
// Tests for capctl()
//==================================================================================================

///
/// # Description
///
/// Tests if [`Capability::ExceptionControl`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_exception_control() -> bool {
    // Attempt to acquire and release exception control capability.
    match nvx::pm::capctl(Capability::ExceptionControl, true) {
        Ok(()) => match nvx::pm::capctl(Capability::ExceptionControl, false) {
            Ok(()) => true,
            _ => false,
        },
        _ => false,
    }
}

///
/// # Description
///
/// Tests if [`Capability::InterruptControl`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_interrupt_control() -> bool {
    // Attempt to acquire and release interrupt control capability.
    match nvx::pm::capctl(Capability::InterruptControl, true) {
        Ok(()) => match nvx::pm::capctl(Capability::InterruptControl, false) {
            Ok(()) => true,
            _ => false,
        },
        _ => false,
    }
}

///
/// # Description
///
/// Tests if [`Capability::IoManagement`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_io_management() -> bool {
    // Attempt to acquire and release I/O management capability.
    match nvx::pm::capctl(Capability::IoManagement, true) {
        Ok(()) => match nvx::pm::capctl(Capability::IoManagement, false) {
            Ok(()) => true,
            _ => false,
        },
        _ => false,
    }
}

///
/// # Description
///
/// Tests if [`Capability::MemoryManagement`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_memory_management() -> bool {
    // Attempt to acquire and release memory management capability.
    match nvx::pm::capctl(Capability::MemoryManagement, true) {
        Ok(()) => match nvx::pm::capctl(Capability::MemoryManagement, false) {
            Ok(()) => true,
            _ => false,
        },
        _ => false,
    }
}

///
/// # Description
///
/// Tests if [`Capability::ProcessManagement`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_process_management() -> bool {
    // Attempt to acquire and release process management capability.
    match nvx::pm::capctl(Capability::ProcessManagement, true) {
        Ok(()) => match nvx::pm::capctl(Capability::ProcessManagement, false) {
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
    match nvx::pm::capctl(Capability::ExceptionControl, true) {
        Ok(()) => match nvx::pm::capctl(Capability::ExceptionControl, true) {
            Ok(()) => return false,
            _ => match nvx::pm::capctl(Capability::ExceptionControl, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire interrupt control capability twice.
    match nvx::pm::capctl(Capability::InterruptControl, true) {
        Ok(()) => match nvx::pm::capctl(Capability::InterruptControl, true) {
            Ok(()) => return false,
            _ => match nvx::pm::capctl(Capability::InterruptControl, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire I/O management capability twice.
    match nvx::pm::capctl(Capability::IoManagement, true) {
        Ok(()) => match nvx::pm::capctl(Capability::IoManagement, true) {
            Ok(()) => return false,
            _ => match nvx::pm::capctl(Capability::IoManagement, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire memory management capability twice.
    match nvx::pm::capctl(Capability::MemoryManagement, true) {
        Ok(()) => match nvx::pm::capctl(Capability::MemoryManagement, true) {
            Ok(()) => return false,
            _ => match nvx::pm::capctl(Capability::MemoryManagement, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire process management capability twice.
    match nvx::pm::capctl(Capability::ProcessManagement, true) {
        Ok(()) => match nvx::pm::capctl(Capability::ProcessManagement, true) {
            Ok(()) => return false,
            _ => match nvx::pm::capctl(Capability::ProcessManagement, false) {
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
    match nvx::pm::capctl(Capability::ExceptionControl, false) {
        Ok(()) => return false,
        _ => (),
    }

    // Attempt to release interrupt control capability without acquiring it.
    match nvx::pm::capctl(Capability::InterruptControl, false) {
        Ok(()) => return false,
        _ => (),
    }

    // Attempt to release I/O management capability without acquiring it.
    match nvx::pm::capctl(Capability::IoManagement, false) {
        Ok(()) => return false,
        _ => (),
    }

    // Attempt to release memory management capability without acquiring it.
    match nvx::pm::capctl(Capability::MemoryManagement, false) {
        Ok(()) => return false,
        _ => (),
    }

    // Attempt to release process management capability without acquiring it.
    match nvx::pm::capctl(Capability::ProcessManagement, false) {
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
    crate::test!(test_capctl_exception_control());
    crate::test!(test_capctl_interrupt_control());
    crate::test!(test_capctl_io_management());
    crate::test!(test_capctl_memory_management());
    crate::test!(test_capctl_process_management());
    crate::test!(test_capctl_invalid_acquire());
    crate::test!(test_capctl_invalid_release());
}
