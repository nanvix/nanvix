// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::{
    fcntl::file_control_request::F_DUPFD,
    ffi::c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Duplicates `fd` into the lowest-numbered available descriptor.
///
/// POSIX `dup(fd)` is exactly `fcntl(fd, F_DUPFD, 0)`: it allocates the lowest free descriptor that
/// aliases `fd`'s open file description, sharing its offset and status flags while starting with
/// close-on-exec cleared. Routing it through `fcntl` reuses the descriptor-table resolution and
/// cache-refresh logic, so the duplication is performed authoritatively by vfsd's flat slot table
/// and works uniformly for console, file, pipe, and host-backed descriptors.
pub fn dup(fd: c_int) -> Result<c_int, Error> {
    ::syslog::trace!("dup(): fd={:?}", fd);
    crate::fcntl::fcntl(fd, F_DUPFD, Some(0))
}
