// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
//==================================================================================================
// Modules
//==================================================================================================

#[cfg(not(feature = "standalone"))]
use crate::fcntl;
use ::sys::error::Error;
#[cfg(not(feature = "standalone"))]
use ::sysapi::fcntl::atflags::AT_FDCWD;
use ::sysapi::{
    ffi::c_int,
    sys_types::mode_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `open()` system call opens the file specified by `pathname`.
///
/// # Parameters
///
/// - `pathname`: Pathname of the file to open.
/// - `flags`:    Flags to open the file.
/// - `mode`:     Mode of the file.
///
/// # Returns
///
/// Upon successful completion, the file descriptor of the file is returned. Otherwise, an error is
/// returned instead.
///
#[allow(unreachable_code)]
pub fn open(pathname: &str, flags: c_int, mode: mode_t) -> Result<c_int, Error> {
    ::syslog::trace!("open(): pathname={:?}, flags={:?}, mode={:?}", pathname, flags, mode);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_open(pathname, flags).map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            ::syslog::error!("open(): VFS open failed (pathname={pathname:?}, error={e})");
            Error::new(code, "vfs open failed")
        })
    }

    #[cfg(not(feature = "standalone"))]
    fcntl::openat(AT_FDCWD, pathname, flags, mode)
}
