// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use ::core::time::Duration;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    time::SystemTime,
};
use sysapi::time::timespec;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Puts the calling thread to sleep.
///
/// # Parameters
///
/// - `req`: The requested sleep time.
/// - `rem`: The remaining time.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn nanosleep(req: &timespec, rem: &mut Option<&mut timespec>) -> Result<(), Error> {
    ::syslog::trace!("nanosleep(): req={:?}, rem={:?}", req, rem);

    // Check if the requested time is valid.
    if req.tv_sec < 0 || req.tv_nsec < 0 || req.tv_nsec >= 1_000_000_000 {
        let reason: &str = "invalid sleep time";
        ::syslog::error!("nanosleep(): {} (tv_sec={}, tv_nsec={})", reason, { req.tv_sec }, {
            req.tv_nsec
        });
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    let secs: u64 = match req.tv_sec.try_into() {
        Ok(secs) => secs,
        Err(_) => {
            let reason: &str = "invalid sleep time";
            ::syslog::error!("nanosleep(): {} (tv_sec={}, tv_nsec={})", reason, { req.tv_sec }, {
                req.tv_nsec
            });
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        },
    };
    let nanos: u32 = match req.tv_nsec.try_into() {
        Ok(nanos) => nanos,
        Err(_) => {
            let reason: &str = "invalid sleep time";
            ::syslog::error!("nanosleep(): {} (tv_sec={}, tv_nsec={})", reason, { req.tv_sec }, {
                req.tv_nsec
            });
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        },
    };

    let duration: Duration = Duration::new(secs, nanos);

    let mut now: SystemTime = SystemTime::default();
    ::sys::kcall::pm::gettime(&mut now)?;

    // Sleep for the requested time.
    ::sys::kcall::pm::sleep(duration)?;

    // Store the remaining time in the provided timespec structure.
    if let Some(rem) = rem {
        let later: SystemTime = match now.checked_add_duration(&duration) {
            Some(later) => later,
            None => {
                let reason: &str = "invalid sleep time";
                ::syslog::error!(
                    "nanosleep(): {} (tv_sec={}, tv_nsec={})",
                    reason,
                    { req.tv_sec },
                    { req.tv_nsec }
                );
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };

        let mut now: SystemTime = SystemTime::default();
        ::sys::kcall::pm::gettime(&mut now)?;

        if now > later {
            rem.tv_sec = 0;
            rem.tv_nsec = 0;
        } else {
            rem.tv_sec = (later.seconds() - now.seconds()) as i64;
            rem.tv_nsec = (later.nanoseconds() - now.nanoseconds()) as i32;
        }
    }

    Ok(())
}
