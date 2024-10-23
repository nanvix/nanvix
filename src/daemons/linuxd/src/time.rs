// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use linuxd::{
    sys::types::clockid_t,
    time::{
        self,
        message::{
            ClockGetResolutionResponse,
            ClockResolutionRequest,
            GetClockTimeRequest,
            GetClockTimeResponse,
        },
    },
};
use nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// do_clock_getres
//==================================================================================================

pub fn do_clock_getres(pid: ProcessIdentifier, request: ClockResolutionRequest) -> Message {
    trace!("clock_getres(): pid={:?}, request={:?}", pid, request);

    let mut res: libc::timespec = LibcTimeSpec::default().into();

    let clk_id: libc::clockid_t = match LibcClockId::try_from(request.clock_id) {
        Ok(clk_id) => clk_id.into(),
        Err(error) => {
            warn!("{:?}", error);
            return crate::build_error(pid, ErrorCode::OperationNotSupported);
        },
    };

    debug!("libc::clock_getres(): clk_id={:?}", clk_id);
    match unsafe { libc::clock_getres(clk_id, &mut res) } {
        0 => {
            let res: time::timespec = LibcTimeSpec(res).into();
            debug!("libc::clock_getres(): {{ tv_sec: {:?}, tv_nsec: {:?} }}", { res.tv_sec }, {
                res.tv_nsec
            });
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

    let mut tp: libc::timespec = LibcTimeSpec::default().into();

    let clk_id: libc::clockid_t = match LibcClockId::try_from(request.clock_id) {
        Ok(clk_id) => clk_id.into(),
        Err(error) => {
            warn!("{:?}", error);
            return crate::build_error(pid, ErrorCode::OperationNotSupported);
        },
    };

    debug!("libc::clock_gettime(): clk_id={:?}", clk_id);
    match unsafe { libc::clock_gettime(clk_id, &mut tp) } {
        0 => {
            let tp: time::timespec = LibcTimeSpec(tp).into();
            debug!("libc::clock_gettime(): {{ tv_sec: {:?}, tv_nsec: {:?} }}", { tp.tv_sec }, {
                tp.tv_nsec
            });
            GetClockTimeResponse::build(pid, tp)
        },
        errno => {
            debug!("libc::clock_gettime(): errno={:?}", errno);
            let error: ErrorCode = ErrorCode::try_from(errno).expect("unknown error code {error}");
            crate::build_error(pid, error)
        },
    }
}

//==================================================================================================
// LibcClockId
//==================================================================================================

/// Wrapper for `libc::clockid_t`.
struct LibcClockId(libc::clockid_t);

impl From<LibcClockId> for libc::clockid_t {
    fn from(clk_id: LibcClockId) -> Self {
        clk_id.0
    }
}

impl LibcClockId {
    fn try_from(clk_id: clockid_t) -> Result<Self, ::nvx::sys::error::Error> {
        match clk_id {
            time::CLOCK_REALTIME => Ok(LibcClockId(libc::CLOCK_REALTIME)),
            time::CLOCK_MONOTONIC => Ok(LibcClockId(libc::CLOCK_MONOTONIC)),
            _ => Err(Error::new(ErrorCode::OperationNotSupported, "unsupported clock_id")),
        }
    }
}

//==================================================================================================
// LibcTimeSpec
//==================================================================================================

/// Wrapper for `libc::timespec`.
struct LibcTimeSpec(libc::timespec);

impl Default for LibcTimeSpec {
    fn default() -> Self {
        LibcTimeSpec(libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        })
    }
}

impl From<LibcTimeSpec> for libc::timespec {
    fn from(tp: LibcTimeSpec) -> Self {
        tp.0
    }
}

impl From<LibcTimeSpec> for time::timespec {
    fn from(tp: LibcTimeSpec) -> Self {
        time::timespec {
            tv_sec: tp.0.tv_sec,
            tv_nsec: tp.0.tv_nsec,
        }
    }
}
