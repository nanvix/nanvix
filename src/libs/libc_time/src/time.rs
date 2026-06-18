// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_int,
    sys_types::{
        clockid_t,
        time_t,
    },
    time::{
        clock_ids::CLOCK_REALTIME,
        timespec,
    },
};

//==================================================================================================
// External Functions
//==================================================================================================

extern "C" {
    fn clock_gettime(clock_id: clockid_t, tp: *mut timespec) -> c_int;
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Stores `result` into `tloc` when it is non-null and returns `result`.
///
/// # Safety
///
/// `tloc` must be null or point to a valid, writable `time_t`.
unsafe fn store_time(result: time_t, tloc: *mut time_t) -> time_t {
    if !tloc.is_null() {
        *tloc = result;
    }
    result
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the current calendar time in seconds since the epoch (1970-01-01 00:00:00 UTC).
///
/// # Parameters
///
/// - `tloc`: If non-null, the return value is also stored in the location pointed to by `tloc`.
///
/// # Returns
///
/// On success, the current time in seconds since the epoch. On error, `(time_t)(-1)`.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `tloc` when non-null and calls
/// an external C function.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn time(tloc: *mut time_t) -> time_t {
    let mut ts: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if clock_gettime(CLOCK_REALTIME, &mut ts) != 0 {
        return -1;
    }
    store_time(ts.tv_sec, tloc)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::store_time;
    use ::sysapi::sys_types::time_t;

    #[test]
    fn test_store_time_writes_when_non_null() {
        // A non-null `tloc` receives a copy of the returned value.
        let mut out: time_t = 0;
        let ret: time_t = unsafe { store_time(12_345 as time_t, &mut out) };
        assert_eq!(ret, 12_345);
        assert_eq!(out, 12_345);
    }

    #[test]
    fn test_store_time_ignores_null() {
        // A null `tloc` is tolerated and the value is still returned.
        let ret: time_t = unsafe { store_time(67_890 as time_t, core::ptr::null_mut()) };
        assert_eq!(ret, 67_890);
    }
}
