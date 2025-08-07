// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::WorkerThreadError;
use ::sys::{
    error::ErrorCode,
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::sys_times::tms;
use ::syscall::sys::times::message::{
    TimesRequest,
    TimesResponse,
};

//==================================================================================================
// do_times()
//==================================================================================================

pub fn do_times(tid: ThreadIdentifier, _request: TimesRequest) -> Result<Message, WorkerThreadError> {
    trace!("times(): tid={tid:?}");

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

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::clock_getres(): errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno).expect("unknown error code {error}");
            Ok(crate::build_error(tid, error))
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

            Ok(TimesResponse::build(tid, elapsed, nanvix_buffer))
        },
    }
}
