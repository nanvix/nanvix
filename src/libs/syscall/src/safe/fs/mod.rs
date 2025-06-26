// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod access_mode_flags;
mod attributes;
mod creation_flags;
mod descriptor_flags;
mod fd;
mod file_type;
mod inode_number;
mod open_flags;
mod path;
mod permissions;
mod status_flags;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl,
    safe::{
        dir::{
            opendir,
            Directory,
            RawDirectory,
        },
        RegularFile,
        RegularFileOpenFlags,
    },
    sys::{
        self,
        stat,
    },
    unistd,
};
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use access_mode_flags::FileAccessModeFlags;
pub use attributes::FileSystemAttributes;
pub use creation_flags::FileCreationFlags;
pub use descriptor_flags::FileDescriptorFlags;
pub use fd::RawFileDescriptor;
pub use file_type::FileType;
pub use inode_number::InodeNumber;
pub use open_flags::OpenFlags;
pub use path::FileSystemPath;
pub use permissions::FileSystemPermissions;
pub use status_flags::FileStatusFlags;
use sysapi::{
    limits::PATH_MAX,
    sys_stat,
    sys_types::{
        c_ssize_t,
        mode_t,
    },
};

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
        chdir(path)
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
        let mut st: sys_stat::stat = sys_stat::stat::default();
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
        flags: &RegularFileOpenFlags,
        permissions: Option<FileSystemPermissions>,
    ) -> Result<RegularFile, Error> {
        let rawfd: RawFileDescriptor = open(pathname, flags, permissions)?;
        Ok(RegularFile::new(rawfd))
    }

    ///
    /// # Description
    ///
    /// Opens a directory.
    ///
    /// # Parameters
    ///
    /// - `directory_name`: The path to the directory to be opened.
    ///
    /// # Returns
    ///
    /// Upon successful completion, a `Directory` object is returned. Otherwise, an error is
    /// returned instead.
    ///
    pub fn open_directory(directory_name: &FileSystemPath) -> Result<Directory, Error> {
        let raw_dir: RawDirectory = opendir(directory_name)?;
        Ok(Directory::new(raw_dir))
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
        unlink(pathname)
    }
}

///
/// # Changes the current working directory.
///
/// # Parameters
///
/// - `path`: The new working directory path.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
pub fn chdir(path: &FileSystemPath) -> Result<(), Error> {
    unistd::chdir(path.as_str())
}

///
/// # Description
///
/// Changes the mode of a file.
///
/// # Parameters
///
/// - `path`:  Pathname of the file.
/// - `mode`:  Mode.
/// - `flag`:  Flag.
///
/// # Returns
///
/// Upon successful completion, the `fchmodat()` system call returns empty. Otherwise, it returns an
/// error.
///
pub fn chmod(pathname: &FileSystemPath, permissions: FileSystemPermissions) -> Result<(), Error> {
    let mode: mode_t = permissions.into();
    sys::stat::chmod(pathname.as_str(), mode)
}

///
/// # Description
///
/// Creates a new hard link to an existing file.
///
/// # Parameters
///
/// - `oladpath`: path to the file to be linked.
/// - `newpath`: path to the new file.
///
/// # Returns
///
/// Upon successful completion, `link()` returns empty. Otherwise, it returns an error.
///
pub fn link(oldpath: &FileSystemPath, newpath: &FileSystemPath) -> Result<(), Error> {
    unistd::link(oldpath.as_str(), newpath.as_str())
}

///
/// # Description
///
/// Retrieves status information of a file without following symbolic links.
///
/// # Parameters
///
/// - `pathname`: Path to the file.
///
/// # Returns
///
/// Upon successful completion, the status information of the file is returned. Otherwise, an
/// error is returned instead.
///
pub fn lstat(pathname: &FileSystemPath) -> Result<FileSystemAttributes, Error> {
    let mut st: sys_stat::stat = sys_stat::stat::default();
    stat::lstat(pathname.as_str(), &mut st)?;
    Ok(FileSystemAttributes::from(st))
}

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
pub fn mkdir(pathname: &FileSystemPath, permissions: FileSystemPermissions) -> Result<(), Error> {
    let mode: mode_t = permissions.into();
    sys::stat::mkdir(pathname.as_str(), mode)
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
    flags: &RegularFileOpenFlags,
    permissions: Option<FileSystemPermissions>,
) -> Result<RawFileDescriptor, Error> {
    let mode: mode_t = match permissions {
        Some(permissions) => permissions.into(),
        None => 0,
    };
    fcntl::syscall::open(pathname.as_str(), flags.into(), mode)
}

///
/// # Description
///
/// Reads the value of a symbolic link.
///
/// # Parameters
///
/// - `path`: The path to the symbolic link.
/// - `buf`: Storage location for the value of the symbolic link.
///
/// # Returns
///
/// Upon successful completion, `readlink()` returns the number of bytes read. Otherwise, it returns
/// an error.
///
pub fn readlink(path: &FileSystemPath) -> Result<FileSystemPath, Error> {
    let mut buf: Vec<u8> = Vec::with_capacity(PATH_MAX);

    let num_bytes_read: c_ssize_t = unistd::readlink(path.as_str(), &mut buf)?;

    let num_bytes_read: usize = match num_bytes_read.try_into() {
        Ok(n) => n,
        Err(_) => return Err(Error::new(ErrorCode::TooBig, "path too long")),
    };

    FileSystemPath::try_from_bytes(&buf[..num_bytes_read])
}

///
/// # Description
///
/// Renames a file.
///
/// # Parameters
///
/// - `oldpath`:  Pathname of the old file.
/// - `newpath`:  Pathname of the new file.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
pub fn rename(oldpath: &FileSystemPath, newpath: &FileSystemPath) -> Result<(), Error> {
    fcntl::syscall::rename(oldpath.as_str(), newpath.as_str())
}

///
/// # Description
///
/// Retrieves status information of a file.
///
/// # Parameters
///
/// - `pathname`: The path to the file whose status is to be retrieved.
///
/// # Returns
///
/// Upon successful completion, the status information of the file is returned. Otherwise, an
/// error is returned instead.
///
pub fn stat(pathname: &FileSystemPath) -> Result<FileSystemAttributes, Error> {
    let mut st: sys_stat::stat = sys_stat::stat::default();
    sys::stat::stat(pathname.as_str(), &mut st)?;
    Ok(FileSystemAttributes::from(st))
}

///
/// # Description
///
/// Creates a symbolic link to an existing file.
///
/// # Parameters
///
/// - `target`: The path to the file to be linked.
/// - `linkpath`: The path to the new symbolic link.
///
/// # Returns
///
/// Upon successful completion, a symbolic link is created and empty is returned. Otherwise, an
/// error is returned instead.
///
pub fn symlink(target: &FileSystemPath, linkpath: &FileSystemPath) -> Result<(), Error> {
    unistd::symlink(target.as_str(), linkpath.as_str())
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
pub fn unlink(pathname: &FileSystemPath) -> Result<(), Error> {
    unistd::unlink(pathname.as_str())
}
