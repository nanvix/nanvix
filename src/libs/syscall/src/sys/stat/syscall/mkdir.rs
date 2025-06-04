// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::AT_FDCWD,
    sys::{
        stat::syscall::mkdirat,
        types::mode_t,
    },
};
use ::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new directory.
///
/// # Parameters
///
/// - `pathname`: Pathname of the new directory.
/// - `mode`: Mode of the new directory.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
pub fn mkdir(pathname: &str, mode: mode_t) -> Result<(), Error> {
    ::syslog::trace!("mkdir(): pathname={pathname:?}, mode={mode:?}");
    mkdirat(AT_FDCWD, pathname, mode)
}
