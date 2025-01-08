// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
};
use ::posix::sys::times::{
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
    trace!("times(): pid={:?}", pid);

    let mut libc_buffer: libc::tms = unsafe { std::mem::zeroed() };

    debug!("libc::times(): buffer={:p}", &libc_buffer as *const libc::tms);

    match unsafe { libc::times(&mut libc_buffer) } {
        -1 => {
            error!("times(): failed errno");
            todo!();
        },
        elapsed => {
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
