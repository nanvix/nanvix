// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::RawFileDescriptor;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Checks if the file descriptor is a terminal.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, a boolean indicating whether the file descriptor is a terminal is
/// returned. Otherwise, an error is returned.
///
pub fn isatty(fd: RawFileDescriptor) -> Result<bool, Error> {
    ::syslog::trace!("isatty(): fd={}", fd);

    // vfsd's flat slot table is authoritative: a descriptor is a terminal if and
    // only if it resolves to a terminal route, so a duplicated or redirected terminal descriptor
    // answers correctly rather than being judged by its number. An unresolvable descriptor is
    // rejected with `EBADF`.
    use crate::fdtable::{
        resolve_result,
        Route,
    };
    match resolve_result(fd)? {
        Some(resolution) if matches!(resolution.route, Route::Console | Route::Terminal) => {
            Ok(true)
        },
        Some(_) => {
            ::syslog::trace!("isatty(): file descriptor is not a terminal (fd={})", fd);
            Ok(false)
        },
        None => {
            ::syslog::trace!("isatty(): invalid file descriptor (fd={})", fd);
            Err(Error::new(ErrorCode::BadFile, "invalid file descriptor"))
        },
    }
}
