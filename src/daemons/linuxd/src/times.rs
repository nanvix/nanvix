// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::ErrorCode,
    ipc::Message,
    pm::ProcessIdentifier,
};
use ::syscall::sys::times::{
    message::{
        TimesRequest,
        TimesResponse,
    },
    tms,
};

//==================================================================================================
// do_times()
//==================================================================================================

pub fn do_times(pid: ProcessIdentifier, _request: TimesRequest) -> Message {
    trace!("times(): pid={pid:?}");

    let mut libc_buffer: libc::tms = libc::tms {
        tms_utime: 0,
        tms_stime: 0,
        tms_cutime: 0,
        tms_cstime: 0,
    };

    debug!("libc::times(): buffer={:p}", &libc_buffer as *const libc::tms);

    match unsafe { libc::times(&mut libc_buffer as *mut libc::tms) } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            error!("libc::clock_getres(): errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno).expect("unknown error code {error}");
            crate::build_error(pid, error)
        },
        elapsed => {
            debug!(
                "libc::times(): elapsed={:?}, tms.utime={:?}, tms.stime={:?}, tms.cutime={:?}, \
                 tms.cstime={:?}",
                elapsed,
                libc_buffer.tms_utime,
                libc_buffer.tms_stime,
                libc_buffer.tms_cutime,
                libc_buffer.tms_cstime
            );

            let nanvix_buffer: tms = tms {
                tms_utime: libc_buffer.tms_utime,
                tms_stime: libc_buffer.tms_stime,
                tms_cutime: libc_buffer.tms_cutime,
                tms_cstime: libc_buffer.tms_cstime,
            };

            TimesResponse::build(pid, elapsed, nanvix_buffer)
        },
    }
}
