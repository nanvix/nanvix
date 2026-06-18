// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::time::timespec;

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

#[cfg_attr(target_arch = "x86_64", allow(clippy::useless_conversion))]
impl TryFrom<LibcTimeSpec> for timespec {
    type Error = Error;

    fn try_from(tp: LibcTimeSpec) -> Result<Self, Self::Error> {
        Ok(timespec {
            tv_sec: tp.0.tv_sec,
            tv_nsec: match tp.0.tv_nsec.try_into() {
                Ok(tv_nsec) => tv_nsec,
                Err(_) => return Err(Error::new(ErrorCode::ValueOutOfRange, "invalid tv_nsec")),
            },
        })
    }
}

#[cfg_attr(target_arch = "x86_64", allow(clippy::useless_conversion))]
impl From<timespec> for LibcTimeSpec {
    fn from(tp: timespec) -> Self {
        LibcTimeSpec(libc::timespec {
            tv_sec: tp.tv_sec,
            tv_nsec: tp.tv_nsec.into(),
        })
    }
}
