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
        posix_dent,
        DirectoryEntryFileType,
    },
    fcntl::{
        self,
        OpenFlags,
    },
    safe::{
        FileSystemPath,
        FileType,
        RawFileDescriptor,
    },
    unistd,
};
use ::core::{
    cell::{
        RefCell,
        RefMut,
    },
    ffi::CStr,
    fmt,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use alloc::{
    rc::Rc,
    string::{
        String,
        ToString,
    },
    vec::Vec,
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
    root: Rc<RefCell<RawDirectoryInner>>,
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
        let inner: RefMut<'_, RawDirectoryInner> = self.root.borrow_mut();
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
/// This structure represents a direcotry in a filesystem.
///
pub struct RawDirectory {
    inner: Rc<RefCell<RawDirectoryInner>>,
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
            inner: Rc::new(RefCell::new(RawDirectoryInner {
                fd,
                directory_name: path.to_string(),
                entries: Vec::new(),
            })),
        }
    }
}

impl fmt::Debug for RawDirectory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner: RefMut<'_, RawDirectoryInner> = self.inner.borrow_mut();
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
    inner: Rc<RefCell<DirectoryInner>>,
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
            inner: Rc::new(RefCell::new(DirectoryInner { dir: raw_dir })),
        }
    }
}

impl Iterator for Directory {
    type Item = Result<DirectoryEntry, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut dir: RefMut<'_, DirectoryInner> = self.inner.borrow_mut();
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
    unistd::close(dir.inner.borrow_mut().fd)
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
    let fd: RawFileDescriptor =
        fcntl::open(directory_name.as_str(), OpenFlags::O_RDONLY | OpenFlags::O_DIRECTORY, 0)?;
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
    let dir_clone: Rc<RefCell<RawDirectoryInner>> = dir.inner.clone();
    let mut inner: RefMut<'_, RawDirectoryInner> = dir.inner.borrow_mut();

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
