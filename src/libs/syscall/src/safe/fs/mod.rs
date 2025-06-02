// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod attributes;
mod fd;
mod file_type;
mod path;
mod permissions;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl,
    safe::{
        RegularFile,
        RegularFileOpenFlags,
    },
    sys::{
        self,
        types::mode_t,
    },
    unistd,
};
use ::alloc::string::String;
use ::sys::error::Error;

//==================================================================================================
// Exports
//==================================================================================================

pub use attributes::FileSystemAttributes;
pub use fd::RawFileDescriptor;
pub use file_type::FileType;
pub use path::FileSystemPath;
pub use permissions::FileSystemPermissions;

//==================================================================================================
// File System
//==================================================================================================

pub struct FileSystem;

impl FileSystem {
    ///
    /// # Description
    ///
    /// Changes the current working directory.
    ///
    /// # Parameters
    ///
    /// - `path`: The new working directory path.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    pub fn change_current_directory(path: &FileSystemPath) -> Result<(), Error> {
        unistd::chdir(path.as_str())
    }

    ///
    /// # Description
    ///
    /// Creates a new regular file in the file system.
    ///
    /// # Parameters
    ///
    /// - `filename`: The name of the file to be created.
    /// - `permissions`: The permissions for the new file.
    ///
    /// # Returns
    ///
    /// Upon successful completion, a regular file is created and returned. Otherwise, an error
    /// is returned instead.
    ///
    pub fn create_regular_file(
        filename: &FileSystemPath,
        permissions: Option<FileSystemPermissions>,
    ) -> Result<RegularFile, Error> {
        let mode: mode_t = match permissions {
            Some(permissions) => permissions.into(),
            None => 0,
        };
        let fd: RawFileDescriptor = fcntl::syscall::creat(filename.as_str(), mode)?;
        Ok(RegularFile::new(fd))
    }

    ///
    /// # Description
    ///
    /// Gets the current working directory.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the current working directory is returned.  Otherwise, an error
    /// is returned instead.
    ///
    pub fn get_current_directory() -> Result<FileSystemPath, Error> {
        // Get the current working directory.
        let path: String = unistd::getcwd()?;
        FileSystemPath::new(&path)
    }

    ///
    /// # Description
    ///
    /// Gets the attributes of a file.
    ///
    /// # Parameters
    ///
    /// - `filename`: The name of the file whose attributes are to be retrieved.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the status information of the file is returned. Otherwise, an
    /// error is returned instead.
    ///
    pub fn get_file_attributes(filename: &FileSystemPath) -> Result<FileSystemAttributes, Error> {
        let mut st: sys::stat::stat = sys::stat::stat::default();
        sys::stat::stat(filename.as_str(), &mut st)?;
        Ok(FileSystemAttributes::from(st))
    }

    ///
    /// # Description
    ///
    /// Opens a regular file in the file system.
    ///
    /// # Parameters
    ///
    /// - `pathname`: The path to the file.
    /// - `flags`: The flags to open the file.
    /// - `permissions`: File permissions when creating a new file.
    ///
    /// # Returns
    ///
    /// Upon successful completion, a `RegularFile` structure is returned. Otherwise, an error is
    /// returned instead.
    ///
    pub fn open_regular_file(
        pathname: &FileSystemPath,
        flags: RegularFileOpenFlags,
        permissions: Option<FileSystemPermissions>,
    ) -> Result<RegularFile, Error> {
        let rawfd: RawFileDescriptor = open(pathname, flags, permissions)?;
        Ok(RegularFile::new(rawfd))
    }

    ///
    /// # Description
    ///
    /// Removes a file from the file system.
    ///
    /// # Parameters
    ///
    /// - `pathname`: The path to the file to be removed.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    pub fn remove_file(pathname: &FileSystemPath) -> Result<(), Error> {
        // Unlink the file.
        match unistd::unlink(pathname.as_str()) {
            Ok(()) => Ok(()),
            Err(error) => Err(error),
        }
    }
}

///
/// # Description
///
/// Opens a file in the file system.
///
/// # Parameters
///
/// - `pathname`: The path to the file.
/// - `flags`: The flags to open the file.
/// - `permissions`: File permissions when creating a new file.
///
/// # Returns
///
/// Upon successful completion, a raw file descriptor is returned. Otherwise, an error is
/// returned instead.
///
pub fn open(
    pathname: &FileSystemPath,
    flags: RegularFileOpenFlags,
    permissions: Option<FileSystemPermissions>,
) -> Result<RawFileDescriptor, Error> {
    let mode: mode_t = match permissions {
        Some(permissions) => permissions.into(),
        None => 0,
    };
    fcntl::syscall::open(pathname.as_str(), flags.into(), mode)
}
