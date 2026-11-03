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
mod tests;
