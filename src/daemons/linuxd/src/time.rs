// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::posix::{
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
            let res: time::timespec = match LibcTimeSpec(res).try_into() {
                Ok(res) => res,
                Err(error) => {
                    warn!("{:?}", error);
                    return crate::build_error(pid, error.code);
                },
            };
            debug!("libc::clock_getres(): {{ tv_sec: {:?}, tv_nsec: {:?} }}", { res.tv_sec }, {
                res.tv_nsec
            });
            ClockGetResolutionResponse::build(pid, res)
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };
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
            let tp: time::timespec = match LibcTimeSpec(tp).try_into() {
                Ok(tp) => tp,
                Err(error) => {
                    return crate::build_error(pid, error.code);
                },
            };
            debug!("libc::clock_gettime(): {{ tv_sec: {:?}, tv_nsec: {:?} }}", { tp.tv_sec }, {
                tp.tv_nsec
            });
            GetClockTimeResponse::build(pid, tp)
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };
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
    fn try_from(clock_id: clockid_t) -> Result<Self, ::nvx::sys::error::Error> {
        match clock_id {
            time::CLOCK_MONOTONIC => Ok(LibcClockId(libc::CLOCK_MONOTONIC)),
            time::CLOCK_PROCESS_CPUTIME_ID => {
                let reason: &str = "CLOCK_PROCESS_CPUTIME_ID is not supported";
                error!("try_from(): {}", reason);
                Err(Error::new(ErrorCode::OperationNotSupported, reason))
            },
            time::CLOCK_THREAD_CPUTIME_ID => {
                let reason: &str = "CLOCK_THREAD_CPUTIME_ID is not supported";
                error!("try_from(): {}", reason);
                Err(Error::new(ErrorCode::OperationNotSupported, reason))
            },
            time::CLOCK_REALTIME => Ok(LibcClockId(libc::CLOCK_REALTIME)),
            clock_id => {
                let reason: &str = "invalid clock_id";
                error!("try_from(): {} (clock_id={:?})", reason, clock_id);
                Err(Error::new(ErrorCode::InvalidArgument, reason))
            },
        }
    }
}

//==================================================================================================
// LibcTimeSpec
//==================================================================================================

/// Wrapper for `libc::timespec`.
pub struct LibcTimeSpec(libc::timespec);

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

impl TryFrom<LibcTimeSpec> for time::timespec {
    type Error = Error;

    fn try_from(tp: LibcTimeSpec) -> Result<Self, Self::Error> {
        Ok(time::timespec {
            tv_sec: tp.0.tv_sec,
            tv_nsec: match tp.0.tv_nsec.try_into() {
                Ok(tv_nsec) => tv_nsec,
                Err(_) => return Err(Error::new(ErrorCode::ValueOutOfRange, "invalid tv_nsec")),
            },
        })
    }
}

impl From<time::timespec> for LibcTimeSpec {
    fn from(tp: time::timespec) -> Self {
        LibcTimeSpec(libc::timespec {
            tv_sec: tp.tv_sec,
            tv_nsec: tp.tv_nsec.into(),
        })
    }
}
