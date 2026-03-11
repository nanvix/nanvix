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

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `unlink()` system call deletes a name from the filesystem.
///
/// If that name was the last link to a file and no processes have the file open, the file is
/// deleted and the space it was using is made available for reuse. have the file open, the file
/// will remain in existence until the last file descriptor referring to it is closed.
///
/// If the name referred to a symbolic link, the link is removed.
///
/// If the name referred to a socket, FIFO, or device, the name for it is removed but processes
/// which have the object open may continue to use it.
///
/// # Parameters
///
/// - `path`: path to the file to be unlinked.
///
/// # Returns
///
/// Upon successful completion, `unlink()` returns empty. Otherwise, it returns an error.
///
#[allow(unreachable_code)]
pub fn unlink(path: &str) -> Result<(), Error> {
    ::syslog::trace!("unlink(): path = {:?}", path);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_unlink(path).map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            ::syslog::error!("unlink(): VFS unlink failed (path={path:?}, error={e})");
            Error::new(code, "vfs unlink failed")
        })
    }

    #[cfg(not(feature = "standalone"))]
    fcntl::unlinkat(AT_FDCWD, path, 0)
}
