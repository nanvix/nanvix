// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::pm::Capability;

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
    match ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, true) {
        Ok(()) => {
            matches!(::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, false), Ok(()))
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
    match ::sys::kcall::pm::__kcall_capctl(Capability::InterruptControl, true) {
        Ok(()) => {
            matches!(::sys::kcall::pm::__kcall_capctl(Capability::InterruptControl, false), Ok(()))
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
    match ::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, true) {
        Ok(()) => {
            matches!(::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, false), Ok(()))
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
    match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, true) {
        Ok(()) => {
            matches!(::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, false), Ok(()))
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
    match ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, true) {
        Ok(()) => {
            matches!(::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, false), Ok(()))
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
    match ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, true) {
        Ok(()) => match ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, true) {
            Ok(()) => return false,
            _ => match ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire interrupt control capability twice.
    match ::sys::kcall::pm::__kcall_capctl(Capability::InterruptControl, true) {
        Ok(()) => match ::sys::kcall::pm::__kcall_capctl(Capability::InterruptControl, true) {
            Ok(()) => return false,
            _ => match ::sys::kcall::pm::__kcall_capctl(Capability::InterruptControl, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire I/O management capability twice.
    match ::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, true) {
        Ok(()) => match ::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, true) {
            Ok(()) => return false,
            _ => match ::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire memory management capability twice.
    match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, true) {
        Ok(()) => match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, true) {
            Ok(()) => return false,
            _ => match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire process management capability twice.
    match ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, true) {
        Ok(()) => match ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, true) {
            Ok(()) => return false,
            _ => match ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, false) {
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
    if let Ok(()) = ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, false) {
        return false;
    }

    // Attempt to release interrupt control capability without acquiring it.
    if let Ok(()) = ::sys::kcall::pm::__kcall_capctl(Capability::InterruptControl, false) {
        return false;
    }

    // Attempt to release I/O management capability without acquiring it.
    if let Ok(()) = ::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, false) {
        return false;
    }

    // Attempt to release memory management capability without acquiring it.
    if let Ok(()) = ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, false) {
        return false;
    }

    // Attempt to release process management capability without acquiring it.
    if let Ok(()) = ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, false) {
        return false;
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
