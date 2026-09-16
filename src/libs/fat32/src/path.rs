// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Resolved FAT paths.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::String;

use crate::Fat32Error;

//==================================================================================================
// Structures
//==================================================================================================

/// A normalized path relative to the root of a FAT filesystem.
///
/// This type proves path syntax only; it carries no VFS mount identity or system-call anchoring
/// provenance. The empty path represents the filesystem root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FatResolvedPath(String);

impl FatResolvedPath {
    /// Creates a resolved FAT path from a mount-local path.
    ///
    /// # Parameters
    ///
    /// - `path`: A normalized path relative to the root of a FAT filesystem.
    ///
    /// # Returns
    ///
    /// A checked resolved path. The empty path represents the filesystem root.
    ///
    /// # Errors
    ///
    /// Returns [`Fat32Error::InvalidPath`] if `path` is absolute, contains a null byte, or contains
    /// empty, current-directory, or parent-directory components.
    ///
    /// # Examples
    ///
    /// If a FAT filesystem is mounted at `/data`, resolving the namespace path
    /// `/data/directory/file` strips the mount prefix before calling this constructor:
    ///
    /// ```
    /// use ::std::string::String;
    /// use ::fat32::{
    ///     Fat32Error,
    ///     FatResolvedPath,
    /// };
    ///
    /// let path: FatResolvedPath = FatResolvedPath::new(String::from("directory/file"))?;
    /// assert_eq!(path.as_str(), "directory/file");
    /// assert_eq!(
    ///     FatResolvedPath::new(String::from("/data/directory/file")),
    ///     Err(Fat32Error::InvalidPath),
    /// );
    /// # Ok::<(), Fat32Error>(())
    /// ```
    pub fn new(path: String) -> Result<Self, Fat32Error> {
        if path.contains('\0')
            || path.starts_with('/')
            || (!path.is_empty()
                && (path.ends_with('/')
                    || path.split('/').any(|component: &str| {
                        component.is_empty() || matches!(component, "." | "..")
                    })))
        {
            return Err(Fat32Error::InvalidPath);
        }

        Ok(Self(path))
    }

    /// Returns the normalized mount-local path.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes this value and returns the normalized mount-local path.
    pub fn into_string(self) -> String {
        self.0
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normalized_paths() {
        for path in [
            "",
            "file",
            "directory/file",
            ".hidden",
            "directory/..hidden",
        ] {
            let resolved: FatResolvedPath = FatResolvedPath::new(String::from(path))
                .expect("normalized FAT path should be accepted");
            assert_eq!(resolved.as_str(), path);
            assert_eq!(resolved.into_string(), path);
        }
    }

    #[test]
    fn rejects_unnormalized_paths() {
        for path in [
            "/file",
            ".",
            "..",
            "directory/./file",
            "directory/../file",
            "directory//file",
            "directory/",
            "file\0name",
        ] {
            assert_eq!(
                FatResolvedPath::new(String::from(path)),
                Err(Fat32Error::InvalidPath),
                "unnormalized FAT path should be rejected: {path:?}",
            );
        }
    }
}
