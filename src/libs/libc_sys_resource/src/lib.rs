// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

use ::config::{
    fds::MAX_OPEN_FDS,
    kernel::{
        MAX_PROCESSES,
        MAX_THREADS,
    },
    memory_layout::{
        USER_BASE_RAW,
        USER_END_RAW,
        USER_HEAP_CAPACITY,
        USER_STACK_SIZE,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    ffi::c_int,
    sys_resource::{
        rlim_t,
        rlimit,
        PRIO_PGRP,
        PRIO_PROCESS,
        PRIO_USER,
        RLIMIT_AS,
        RLIMIT_CORE,
        RLIMIT_CPU,
        RLIMIT_DATA,
        RLIMIT_FSIZE,
        RLIMIT_MEMLOCK,
        RLIMIT_NOFILE,
        RLIMIT_NPROC,
        RLIMIT_RSS,
        RLIMIT_STACK,
        RLIM_INFINITY,
    },
};
use ::syscall::errno::__errno_location;
use ::syslog::trace_syscall;

//==================================================================================================
// Constants
//==================================================================================================

/// Size of the user address space, in which every mapping of a process lies.
const USER_ADDRESS_SPACE_SIZE: usize = USER_END_RAW - USER_BASE_RAW;

// Resource limits are computed as `usize` values and then cast to `rlim_t`, which is lossless only
// if `rlim_t` is at least as wide as `usize` and the descriptor bound is non-negative.
::static_assert::assert_eq!(::core::mem::size_of::<usize>() <= ::core::mem::size_of::<rlim_t>());
::static_assert::assert_eq!(MAX_OPEN_FDS >= 0);

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn valid_priority_selector(which: c_int) -> bool {
    matches!(which, PRIO_PROCESS | PRIO_PGRP | PRIO_USER)
}

///
/// # Description
///
/// Looks up the limit that Nanvix enforces on the consumption of a resource.
///
/// # Parameters
///
/// - `resource`: Resource to look up.
///
/// # Returns
///
/// On success, returns the limit on `resource`, which is [`RLIM_INFINITY`] if Nanvix does not
/// limit that resource. Otherwise, returns an error.
///
/// # Errors
///
/// - [`ErrorCode::InvalidArgument`]: `resource` is not a valid resource.
///
fn resource_limit(resource: c_int) -> Result<rlim_t, Error> {
    let limit: usize = match resource {
        // Nanvix never writes core images.
        RLIMIT_CORE => 0,
        // The heap cannot grow past its capacity, so `malloc()` fails once it is exhausted.
        RLIMIT_DATA => USER_HEAP_CAPACITY,
        // The stack of the initial thread is demand-paged up to this size and cannot grow past it.
        RLIMIT_STACK => USER_STACK_SIZE,
        // Descriptors are assigned in `[0, MAX_OPEN_FDS)`.
        RLIMIT_NOFILE => MAX_OPEN_FDS as usize,
        // The kernel caps the number of processes and threads that may exist at the same time, and
        // every process has at least one thread, so both caps bound the number of processes.
        RLIMIT_NPROC => MAX_PROCESSES.min(MAX_THREADS),
        // Every mapping of a process lies in the user address space.
        RLIMIT_AS => USER_ADDRESS_SPACE_SIZE,
        // Nanvix does not limit CPU time, file sizes, resident set sizes, or locked memory.
        RLIMIT_CPU | RLIMIT_FSIZE | RLIMIT_RSS | RLIMIT_MEMLOCK => return Ok(RLIM_INFINITY),
        _ => return Err(Error::new(ErrorCode::InvalidArgument, "invalid resource")),
    };

    Ok(limit as rlim_t)
}

///
/// # Description
///
/// Gets the limits on the consumption of a resource.
///
/// Nanvix does not support changing resource limits, so the soft and hard limits on a resource are
/// the same: the limit that the system enforces on that resource, or [`RLIM_INFINITY`] if it
/// enforces none.
///
/// # Parameters
///
/// - `resource`: Resource to query.
/// - `rlim`: Storage location for the limits on `resource`.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, `-1` is returned, `errno` is set to
/// indicate the error, and the storage pointed to by `rlim` is left unchanged.
///
/// # Errors
///
/// - `EINVAL`: `resource` is not a valid resource, or `rlim` is a null pointer.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer.
///
/// It is safe to call this function if `rlim` is either a null pointer or points to a valid
/// [`rlimit`] structure.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/getrlimit.html
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn getrlimit(resource: c_int, rlim: *mut rlimit) -> c_int {
    // Look up the limit on `resource` and check for errors.
    let limit: rlim_t = match resource_limit(resource) {
        Ok(limit) => limit,
        Err(error) => {
            ::syslog::warn!("getrlimit(): {} (resource={})", error.reason, resource);
            unsafe {
                *__errno_location() = error.code.get();
            }
            return -1;
        },
    };

    // Check if `rlim` is not valid.
    if rlim.is_null() {
        ::syslog::warn!("getrlimit(): rlim is null (resource={})", resource);
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    // Limits cannot be changed, so the soft limit is the hard limit.
    unsafe {
        *rlim = rlimit {
            rlim_cur: limit,
            rlim_max: limit,
        };
    }

    0
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
