// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use ::alloc::{
    ffi::CString,
    string::{
        String,
        ToString,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::limits::PATH_MAX;

//==================================================================================================
// Path
//==================================================================================================

///
/// # Description
///
/// A structure that represents a path in the file system.
///
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FileSystemPath {
    name: String,
}

impl FileSystemPath {
    /// File path separator.
    pub const SEPARATOR: char = '/';

    ///
    /// # Description
    ///
    /// Creates a new path from a string.
    ///
    /// # Parameters
    ///
    /// - `name`: The name of the path.
    ///
    /// # Returns
    ///
    /// Upon successful completion, a `FileSystemPath` structure is returned. Otherwise, an error is
    /// returned instead.
    ///
    pub fn new(name: &str) -> Result<FileSystemPath, Error> {
        // Check if path is empty.
        if name.is_empty() {
            let reason: &str = "empty path";
            ::syslog::error!("new(): {reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if path is invalid.
        let name_cstr: CString = match CString::new(name) {
            Ok(cstr) => cstr,
            Err(_) => {
                let reason: &str = "invalid path";
                ::syslog::error!("new(): {reason}");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };

        // Check if path is too long.
        if name_cstr.as_bytes().len() > PATH_MAX {
            let reason: &str = "path is too long";
            ::syslog::error!("new(): {reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        Ok(FileSystemPath {
            name: name.to_string(),
        })
    }

    ///
    /// # Description
    ///
    /// Casts `self` to a `str`.
    ///
    /// # Returns
    ///
    /// The path as a reference to a string.
    ///
    pub fn as_str(&self) -> &str {
        &self.name
    }

    ///
    /// # Description
    ///
    /// Converts `self` to a byte slice.
    ///
    /// # Returns
    ///
    /// The path as a byte slice.
    ///
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        match CString::from_vec_with_nul(bytes.to_vec()) {
            Ok(name) => match name.into_string() {
                Ok(name) => FileSystemPath::new(&name),
                Err(_) => {
                    let reason: &str = "invalid UTF-8 sequence in path";
                    ::syslog::error!("try_from_bytes(): {reason}");
                    Err(Error::new(ErrorCode::InvalidArgument, reason))
                },
            },
            Err(_) => {
                let reason: &str = "invalid path bytes";
                ::syslog::error!("try_from_bytes(): {reason}");
                Err(Error::new(ErrorCode::InvalidArgument, reason))
            },
        }
    }

    ////
    /// # Description
    ///
    /// Joins two paths into a single path.
    ///
    /// # Parameters
    ///
    /// - `self`: The first path.
    /// - `other`: The second path.
    ///
    /// # Returns
    ///
    /// Upon successful completion, a new `FileSystemPath` structure is returned. Otherwise, an
    /// error is returned instead.
    ///
    pub fn join(&self, other: &FileSystemPath) -> Result<FileSystemPath, Error> {
        ::syslog::debug!("join(): {} + {}", self.name, other.name);
        // Check if the other path is empty.
        if other.name.is_empty() {
            let reason: &str = "cannot join with an empty path";
            ::syslog::error!("join(): {reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if resulting path would be too long.
        let joined_length: usize = self.name.len() + other.name.len() + 1; // +1 for the separator
        if joined_length > PATH_MAX {
            let reason: &str = "resulting path is too long";
            ::syslog::error!("join(): {reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Join the paths.
        let joined_name: String = alloc::format!("{}{}{}", self.name, Self::SEPARATOR, other.name);
        FileSystemPath::new(&joined_name)
    }
}
