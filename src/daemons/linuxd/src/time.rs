// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use linuxd::time::{
    self,
    message::{
        ClockGetResolutionResponse,
        ClockResolutionRequest,
        GetClockTimeRequest,
        GetClockTimeResponse,
    },
};
use nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
};

//==================================================================================================
// do_clock_getres
//==================================================================================================

pub fn do_clock_getres(pid: ProcessIdentifier, request: ClockResolutionRequest) -> Message {
    trace!("clock_getres(): pid={:?}, request={:?}", pid, request);

    let mut res: libc::timespec = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    let clk_id: libc::clockid_t = request.clock_id;

    if !linuxd::time::__is_clock_id_supported(clk_id) {
        warn!("unsupported clock_id {:?}", clk_id);
        return crate::build_error(pid, ErrorCode::OperationNotSupported);
    }

    debug!("libc::clock_getres(): clk_id={:?}", clk_id);
    match unsafe { libc::clock_getres(clk_id, &mut res) } {
        0 => {
            let res: time::timespec = time::timespec {
                tv_sec: res.tv_sec,
                tv_nsec: res.tv_nsec,
            };
            debug!(
                "libc::clock_getres(): {{ tv_sec: {:?}, tv_nsec: {:?} }}",
                res.tv_sec, res.tv_nsec
            );
            ClockGetResolutionResponse::build(pid, res)
        },
        errno => {
            debug!("libc::clock_getres(): errno={:?}", errno);
            let error: ErrorCode = ErrorCode::try_from(errno).expect("unknown error code {error}");
            crate::build_error(pid, error)
        },
    }
}

//==================================================================================================
// do_clock_gettime
//==================================================================================================

pub fn do_clock_gettime(pid: ProcessIdentifier, request: GetClockTimeRequest) -> Message {
    trace!("clock_gettime(): pid={:?}, request={:?}", pid, request);

    let mut tp: libc::timespec = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    let clk_id: libc::clockid_t = request.clock_id;

    if !linuxd::time::__is_clock_id_supported(clk_id) {
        warn!("unsupported clock_id {:?}", clk_id);
        return crate::build_error(pid, ErrorCode::OperationNotSupported);
    }

    debug!("libc::clock_gettime(): clk_id={:?}", clk_id);
    match unsafe { libc::clock_gettime(clk_id, &mut tp) } {
        0 => {
            let tp: time::timespec = time::timespec {
                tv_sec: tp.tv_sec,
                tv_nsec: tp.tv_nsec,
            };
            debug!(
                "libc::clock_gettime(): {{ tv_sec: {:?}, tv_nsec: {:?} }}",
                tp.tv_sec, tp.tv_nsec
            );
            GetClockTimeResponse::build(pid, tp)
        },
        errno => {
            debug!("libc::clock_gettime(): errno={:?}", errno);
            let error: ErrorCode = ErrorCode::try_from(errno).expect("unknown error code {error}");
            crate::build_error(pid, error)
        },
    }
}
