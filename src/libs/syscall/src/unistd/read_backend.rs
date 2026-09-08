// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Read-backend classification without guest runtime dependencies.

//==================================================================================================
// Imports
//==================================================================================================

use crate::fd_route::Route;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Enumerations
//==================================================================================================

/// Read backend; transport and cancellation follow from the variant.
#[derive(Clone, Copy)]
pub(super) enum ReadBackend {
    /// Direct console input without VFSD.
    #[cfg_attr(
        not(feature = "syscall"),
        expect(
            dead_code,
            reason = "Constructed only by the guest read implementation."
        )
    )]
    KernelConsole,
    /// Terminal input parked in VFSD's console wait table.
    VfsConsole,
    /// File or pipe input served by VFSD.
    Vfs,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl TryFrom<Route> for ReadBackend {
    type Error = Error;

    fn try_from(route: Route) -> Result<Self, Self::Error> {
        match route {
            Route::Console | Route::Terminal => Ok(Self::VfsConsole),
            Route::Vfs => Ok(Self::Vfs),
            Route::Socket => Err(Error::new(ErrorCode::BadFile, "read: unsupported socket route")),
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::ReadBackend;
    use crate::fd_route::Route;
    use ::sys::error::ErrorCode;

    #[test]
    fn terminal_reads_use_console_backend() {
        for route in [Route::Console, Route::Terminal] {
            assert!(matches!(ReadBackend::try_from(route), Ok(ReadBackend::VfsConsole)));
        }
    }

    #[test]
    fn other_vfs_reads_use_vfs_backend() {
        assert!(matches!(ReadBackend::try_from(Route::Vfs), Ok(ReadBackend::Vfs)));
    }

    #[test]
    fn sockets_are_not_read_backends() {
        assert!(matches!(
            ReadBackend::try_from(Route::Socket),
            Err(error) if error.code == ErrorCode::BadFile
        ));
    }
}
