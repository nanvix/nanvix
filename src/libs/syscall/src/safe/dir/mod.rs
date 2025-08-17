// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::panic)]
#![forbid(unsafe_code)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dirent::{
        self,
        DirectoryEntryFileType,
    },
    fcntl::{
        self,
    },
    safe::{
        fs::InodeNumber,
        FileSystemPath,
        FileType,
        OpenFlags,
        RawFileDescriptor,
    },
    unistd,
};
use ::core::{
    ffi::CStr,
    fmt,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    dirent::posix_dent,
    ffi::c_int,
};
use alloc::{
    string::{
        String,
        ToString,
    },
    sync::Arc,
    vec::Vec,
};
use spin::{
    Mutex,
    MutexGuard,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Minimum number of entries to get when refilling buffers.
const REFILL_COUNT: usize = 1;

//==================================================================================================
// RawDirectoryEntry
//==================================================================================================

///
/// # Description
///
/// This structure represents a directory entry in a filesystem.
///
pub struct RawDirectoryEntry {
    /// The underlying directory entry.
    entry: posix_dent,
    /// Root directory of the entry.
    root: Arc<Mutex<RawDirectoryInner>>,
}

impl RawDirectoryEntry {
    ///
    /// # Description
    ///
    /// Returns the file type of the directory entry.
    ///
    /// # Returns
    ///
    /// Returns the file type of the directory entry.
    ///
    pub fn file_type(&self) -> FileType {
        FileType::from(DirectoryEntryFileType::from(self.entry.d_type))
    }

    ///
    /// # Description
    ///
    /// Returns the inode number of the directory entry.
    ///
    /// # Returns
    ///
    /// The inode number of the directory entry.
    ///
    pub fn inode_number(&self) -> InodeNumber {
        InodeNumber::from(self.entry.d_ino)
    }

    ///
    /// # Description
    ///
    /// Returns the name of the directory entry as a byte slice.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the name of the directory entry as a byte slice is returned.
    ///
    pub fn file_name_bytes(&self) -> Result<&[u8], Error> {
        // Coerce the byte slice to a C string.
        match CStr::from_bytes_until_nul(&self.entry.d_name) {
            Ok(cstr) => Ok(cstr.to_bytes()),
            Err(_error) => {
                let reason: &str = "invalid C string in directory entry name";
                Err(Error::new(ErrorCode::ValueOutOfRange, reason))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Returns the name of the directory entry.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the name of the directory entry is returned. Otherwise, an
    /// error is returned instead.
    ///
    pub fn file_name(&self) -> Result<&str, Error> {
        // Coerce the byte slice to a C string.
        let file_name_cstr: &CStr = match CStr::from_bytes_until_nul(&self.entry.d_name) {
            Ok(cstr) => cstr,
            Err(_error) => {
                let reason: &str = "invalid C string in directory entry name";
                return Err(Error::new(ErrorCode::ValueOutOfRange, reason));
            },
        };

        // Coerce the C string to a string.
        match file_name_cstr.to_str() {
            Ok(name) => Ok(name),
            Err(_error) => {
                let reason: &str = "invalid UTF-8 sequence in directory entry name";
                Err(Error::new(ErrorCode::ValueOutOfRange, reason))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Returns the directory name to the directory entry.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the directory name to the directory entry is returned.
    /// Otherwise, an error is returned instead.
    ///
    pub fn directory_name(&self) -> String {
        let inner: MutexGuard<'_, RawDirectoryInner> = self.root.lock();
        inner.directory_name.to_string()
    }

    ///
    /// # Description
    ///
    /// Returns the full path to the directory entry.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the full path to the directory entry is returned.  Otherwise, an
    /// error is returned instead.
    ///
    pub fn path(&self) -> Result<String, Error> {
        let name: &str = self.file_name()?;
        let directory_name: String = self.directory_name();
        if directory_name.is_empty() {
            Ok(name.to_string())
        } else {
            Ok(::alloc::format!("{}/{}", directory_name, name))
        }
    }
}

impl fmt::Debug for RawDirectoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RawDirectoryEntry {{ name: {:?}, file_type: {:?} }}",
            self.file_name(),
            self.file_type()
        )
    }
}

//==================================================================================================
// RawDirectory
//==================================================================================================

struct RawDirectoryInner {
    /// Underlying file descriptor.
    fd: RawFileDescriptor,
    /// Path to the directory.
    directory_name: String,
    /// Buffered directory entries.
    entries: Vec<RawDirectoryEntry>,
}

///
/// # Description
///
/// This structure represents a directory in a filesystem.
///
pub struct RawDirectory {
    inner: Arc<Mutex<RawDirectoryInner>>,
}

impl RawDirectory {
    ///
    /// # Description
    ///
    /// Creates a new raw directory.
    ///
    /// # Parameters
    ///
    /// - `fd`: The file descriptor of the directory.
    /// - `path`: The path to the directory.
    ///
    /// # Returns
    ///
    /// A new raw directory.
    ///
    pub fn new(fd: RawFileDescriptor, path: &str) -> Self {
        RawDirectory {
            inner: Arc::new(Mutex::new(RawDirectoryInner {
                fd,
                directory_name: path.to_string(),
                entries: Vec::new(),
            })),
        }
    }
}

impl fmt::Debug for RawDirectory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner: MutexGuard<'_, RawDirectoryInner> = self.inner.lock();
        write!(f, "RawDirectory {{ fd: {}, path: {:?} }}", inner.fd, inner.directory_name)
    }
}

//==================================================================================================
// DirectoryEntry
//==================================================================================================

///
/// # Description
///
/// This structure represents a directory entry in a filesystem.
///
pub struct DirectoryEntry {
    entry: RawDirectoryEntry,
}

impl DirectoryEntry {
    ///
    /// # Description
    ///
    /// Returns the name of the directory entry.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the name of the directory entry is returned. Otherwise, an error
    /// is returned instead.
    ///
    pub fn file_name(&self) -> Result<&str, Error> {
        self.entry.file_name()
    }

    ///
    /// # Description
    ///
    /// Returns the file type of the directory entry.
    ///
    /// # Returns
    ///
    /// The file type of the directory entry.
    ///
    pub fn file_type(&self) -> FileType {
        self.entry.file_type()
    }
}

//==================================================================================================
// Directory
//==================================================================================================

///
/// # Description
///
/// This structure represents the inner state of a directory in a filesystem.
///
struct DirectoryInner {
    /// The underlying raw directory.
    dir: RawDirectory,
}

impl Drop for DirectoryInner {
    fn drop(&mut self) {
        if let Err(error) = closedir(&self.dir) {
            ::syslog::error!("DirectoryInner::drop(): {error:?}");
        }
    }
}

///
/// # Description
///
/// This structure represents a directory in a filesystem.
///
pub struct Directory {
    inner: Arc<Mutex<DirectoryInner>>,
}

impl Directory {
    ///
    /// # Description
    ///
    /// Creates a new directory from a raw directory.
    ///
    /// # Parameters
    ///
    /// - `dir`: The raw directory to be wrapped.
    ///
    /// # Returns
    ///
    /// A new directory.
    ///
    pub fn new(raw_dir: RawDirectory) -> Self {
        Directory {
            inner: Arc::new(Mutex::new(DirectoryInner { dir: raw_dir })),
        }
    }
}

impl Iterator for Directory {
    type Item = Result<DirectoryEntry, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut dir: MutexGuard<'_, DirectoryInner> = self.inner.lock();
        match readdir(&mut dir.dir) {
            Ok(Some(raw_entry)) => Some(Ok(DirectoryEntry { entry: raw_entry })),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Closes a directory.
///
/// # Parameters
///
/// - `dir`: A reference to the raw directory to be closed.
///
/// # Returns
///
/// Upon successful completion, `Ok(())` is returned. Otherwise, an error is returned.
///
pub fn closedir(dir: &RawDirectory) -> Result<(), Error> {
    unistd::close(dir.inner.lock().fd)
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
/// Upon successful completion, a directory is returned. Otherwise, an error is returned instead.
///
pub fn opendir(directory_name: &FileSystemPath) -> Result<RawDirectory, Error> {
    let flags: c_int = OpenFlags::read_only().set_directory(true).into();
    let fd: RawFileDescriptor = fcntl::open(directory_name.as_str(), flags, 0)?;
    Ok(RawDirectory::new(fd, directory_name.as_str()))
}

///
/// # Description
///
/// Reads the next entry from a directory.
///
/// # Parameters
///
/// - `dir`: A mutable reference to the raw directory from which to read the entry.
///
/// # Returns
///
/// Upon successful completion, a directory is returned. If there are no more entries to read,
/// `None` is returned..  If an error occurs, an error is returned instead
///
pub fn readdir(dir: &mut RawDirectory) -> Result<Option<RawDirectoryEntry>, Error> {
    let dir_clone: Arc<Mutex<RawDirectoryInner>> = dir.inner.clone();
    let mut inner: MutexGuard<'_, RawDirectoryInner> = dir.inner.lock();

    if let Some(entry) = inner.entries.pop() {
        return Ok(Some(entry));
    }

    // Refill the entries buffer.
    let mut entries: Vec<posix_dent> = dirent::posix_getdents(inner.fd, REFILL_COUNT)?;
    if entries.is_empty() {
        return Ok(None);
    }

    // Push entries to buffer.
    while let Some(entry) = entries.pop() {
        inner.entries.push(RawDirectoryEntry {
            entry,
            root: dir_clone.clone(),
        });
    }

    // Return the next entry.
    Ok(inner.entries.pop())
}
