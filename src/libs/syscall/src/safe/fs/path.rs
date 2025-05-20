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

use crate::limits;

//==================================================================================================
// Path
//==================================================================================================

///
/// # Description
///
/// A structure that represents a path in the file system.
///
#[derive(Debug)]
pub struct FileSystemPath {
    name: String,
}

impl FileSystemPath {
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
        if name_cstr.as_bytes().len() > limits::PATH_MAX {
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
}
