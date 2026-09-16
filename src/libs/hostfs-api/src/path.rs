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

/// Mount point at which the host filesystem is exposed in the VFS namespace.
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
    /// Resolves a VFS namespace path beneath the host filesystem mount.
    ///
    /// # Parameters
    ///
    /// - `path`: A path in the VFS namespace.
    ///
    /// # Returns
    ///
    /// A host-relative path with exactly one mount boundary removed. An exact mount match produces
    /// an empty path.
    ///
    /// # Errors
    ///
    /// Returns [`HOSTFS_ERR_INVALID`] if `path` is outside the host filesystem mount or contains an
    /// embedded null byte.
    pub fn from_namespace_path(path: &str) -> Result<Self, i32> {
        let relative: &str = if path == HOSTFS_MOUNT_PATH {
            ""
        } else {
            path.strip_prefix(HOSTFS_MOUNT_PATH)
                .and_then(|suffix: &str| suffix.strip_prefix('/'))
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
mod tests {
    use super::*;

    #[test]
    fn resolves_hostfs_namespace_paths() {
        for (path, expected) in [
            ("/mnt", ""),
            ("/mnt/file", "file"),
            ("/mnt//file", "/file"),
            ("/mnt/./file", "./file"),
            ("/mnt/directory/../file", "directory/../file"),
            ("/mnt/directory//file", "directory//file"),
        ] {
            let resolved: HostResolvedPath = HostResolvedPath::from_namespace_path(path)
                .expect("hostfs namespace path should be accepted");
            assert_eq!(resolved.as_str(), expected);
            assert_eq!(resolved.into_string(), expected);
        }
    }

    #[test]
    fn rejects_paths_outside_hostfs_mount() {
        for path in ["", "/", "/mntfoo", "/mntfoo/file"] {
            assert_eq!(
                HostResolvedPath::from_namespace_path(path),
                Err(HOSTFS_ERR_INVALID),
                "path outside hostfs mount should be rejected: {path:?}",
            );
        }
    }

    #[test]
    fn checks_wire_paths_without_normalizing_them() {
        for path in ["", "file", "/file", "./file", "directory/../file", "a//b"] {
            let resolved: HostResolvedPath = HostResolvedPath::from_wire(String::from(path))
                .expect("valid hostfs wire path should be accepted");
            assert_eq!(resolved.as_str(), path);
        }

        assert_eq!(
            HostResolvedPath::from_wire(String::from("file\0name")),
            Err(HOSTFS_ERR_INVALID),
        );
    }

    #[test]
    fn rejects_malformed_wire_bytes() {
        assert_eq!(HostResolvedPath::from_wire_bytes(&[0xff]), Err(HOSTFS_ERR_INVALID),);
        assert_eq!(HostResolvedPath::from_wire_bytes(b"file\0name"), Err(HOSTFS_ERR_INVALID),);

        let resolved: HostResolvedPath = HostResolvedPath::from_wire_bytes(b"a/../b")
            .expect("valid UTF-8 hostfs path should be accepted");
        assert_eq!(resolved.as_str(), "a/../b");
    }
}
