// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Resolved host filesystem paths.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::String;

use crate::HOSTFS_ERR_INVALID;

//==================================================================================================
// Constants
//==================================================================================================

/// Guest mount point at which the host filesystem is exposed.
pub const HOSTFS_MOUNT_PATH: &str = "/mnt";

//==================================================================================================
// Structures
//==================================================================================================

/// A checked path passed to host filesystem operations.
///
/// This type proves only that the path contains no null byte; it carries no VFS mount or descriptor
/// provenance. Path spelling is preserved for host filesystem resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostResolvedPath(String);

impl HostResolvedPath {
    /// Converts a guest path beneath `/mnt` to a host-relative path.
    ///
    /// This strips exactly one guest mount prefix and rejects null bytes. It does not check whether
    /// hostfs is enabled, look up an object, normalize components, or resolve symlinks.
    ///
    /// For example, `/mnt` and `/mnt/` become `""`, `/mnt/file` becomes `file`, and
    /// `/mnt/dir/../child//` becomes `dir/../child//`. `/mntfoo/file` is rejected.
    ///
    /// # Parameters
    ///
    /// - `path`: An absolute guest path at or beneath the hostfs mount point.
    ///
    /// # Returns
    ///
    /// A host-relative path with the mount prefix removed and all remaining spelling preserved.
    ///
    /// # Errors
    ///
    /// Returns [`HOSTFS_ERR_INVALID`] if `path` is outside the host filesystem mount or contains an
    /// embedded null byte.
    pub fn from_guest_path(path: &str) -> Result<Self, i32> {
        let relative = if path == HOSTFS_MOUNT_PATH {
            ""
        } else {
            path.strip_prefix(HOSTFS_MOUNT_PATH)
                .and_then(|suffix| suffix.strip_prefix('/'))
                .ok_or(HOSTFS_ERR_INVALID)?
        };

        Self::from_wire(String::from(relative))
    }

    /// Checks a host-relative path decoded from the wire.
    ///
    /// # Parameters
    ///
    /// - `path`: A decoded host-relative path. Its spelling is preserved.
    ///
    /// # Returns
    ///
    /// A checked host-relative path.
    ///
    /// # Errors
    ///
    /// Returns [`HOSTFS_ERR_INVALID`] if `path` contains an embedded null byte.
    pub fn from_wire(path: String) -> Result<Self, i32> {
        if path.contains('\0') {
            return Err(HOSTFS_ERR_INVALID);
        }

        Ok(Self(path))
    }

    /// Decodes and checks a host-relative path from wire bytes.
    ///
    /// # Parameters
    ///
    /// - `bytes`: UTF-8 bytes containing a host-relative path.
    ///
    /// # Returns
    ///
    /// A checked host-relative path whose spelling matches `bytes`.
    ///
    /// # Errors
    ///
    /// Returns [`HOSTFS_ERR_INVALID`] if `bytes` is not valid UTF-8 or contains an embedded null
    /// byte.
    pub fn from_wire_bytes(bytes: &[u8]) -> Result<Self, i32> {
        let path: &str = ::core::str::from_utf8(bytes).map_err(|_| HOSTFS_ERR_INVALID)?;
        Self::from_wire(String::from(path))
    }

    /// Returns the host-relative path.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes this value and returns the host-relative path.
    pub fn into_string(self) -> String {
        self.0
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
