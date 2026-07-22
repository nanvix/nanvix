// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_resource::{
        rlimit,
        PRIO_PGRP,
        PRIO_PROCESS,
        PRIO_USER,
    },
};
use ::syscall::errno::__errno_location;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn valid_priority_selector(which: c_int) -> bool {
    matches!(which, PRIO_PROCESS | PRIO_PGRP | PRIO_USER)
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn getrlimit(_resource: c_int, _rlim: *mut rlimit) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/459
    ::syslog::debug!("getrlimit(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn setrlimit(_resource: c_int, _rlim: *const rlimit) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/469
    ::syslog::debug!("setrlimit(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn getpriority(which: c_int, _who: c_int) -> c_int {
    if !valid_priority_selector(which) {
        ::syslog::warn!("getpriority(): invalid priority selector (which={})", which);
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    // Nanvix has no process-scheduling-priority concept; report the normal
    // priority (0). Callers (nice, renice, start-stop-daemon) treat 0 as the
    // default nice value.
    ::syslog::debug!("getpriority(): not implemented; reporting normal priority");
    0
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn setpriority(which: c_int, _who: c_int, _prio: c_int) -> c_int {
    if !valid_priority_selector(which) {
        ::syslog::warn!("setpriority(): invalid priority selector (which={})", which);
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    // Nanvix has no process-scheduling-priority concept; accept and ignore the
    // request so that nice/renice succeed as no-ops rather than hard-failing.
    ::syslog::debug!("setpriority(): not implemented; ignoring");
    0
}
