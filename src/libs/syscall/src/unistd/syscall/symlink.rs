// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(not(feature = "standalone"))]
use crate::unistd;
use ::sys::error::Error;
#[cfg(feature = "standalone")]
use ::sys::error::ErrorCode;
#[cfg(not(feature = "standalone"))]
use ::sysapi::fcntl::atflags::AT_FDCWD;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `symlink()` system call creates a symbolic link named `linkpath` which contains the string
/// `target`.
///
/// Symbolic links are interpreted at run-time as if the contents of the link had been substituted
/// into the path being followed to find a file or directory.
///
/// Symbolic links may contain `..` path components, which refer to the parent directory of the
/// symbolic link.
///
/// A symbolic link may point to an existing file or to a non-existing file.
///
/// The permissions of a symbolic link are not used. The permissions of the file it points to are
/// used instead.
///
/// If the `linkpath` exists, it will not be overwritten.
///
/// # Parameters
///
/// - `target`: path to the file to be linked.
/// - `linkpath`: path to the new file.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, an error code is returned instead.
///
pub fn symlink(target: &str, linkpath: &str) -> Result<(), Error> {
    ::syslog::trace!("symlink(): target = {:?}, linkpath = {:?}", target, linkpath);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::syslog::error!("symlink(): symlinks not supported on VFS (linkpath={linkpath:?})");
        Err(Error::new(ErrorCode::OperationNotSupported, "symbolic links not supported on VFS"))
    }

    #[cfg(not(feature = "standalone"))]
    unistd::symlinkat(target, AT_FDCWD, linkpath)
}
