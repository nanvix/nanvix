// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Synthetic `/dev` namespace.

//==================================================================================================
// Imports
//==================================================================================================

use crate::mount::{
    anchor_path,
    normalize_anchored,
};
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::fat32::Fat32Error;

//==================================================================================================
// Constants
//==================================================================================================

/// Synthetic device identifier for the device namespace.
const DEVICE_NAMESPACE_ID: u64 = 4;

/// Stable inode identifier for `/dev`.
pub(crate) const DIRECTORY_INODE: u64 = 1;

//==================================================================================================
// Structures
//==================================================================================================

/// Metadata for a synthetic device-namespace entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeviceMetadata {
    /// Synthetic device identifier.
    device: u64,
    /// Stable inode identifier.
    inode: u64,
    /// Whether this entry is a directory.
    is_directory: bool,
}

//==================================================================================================
// Enumerations
//==================================================================================================

/// A path resolved within the synthetic device namespace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DevicePath {
    /// The synthetic `/dev` directory.
    Directory,
    /// An unknown entry below `/dev`.
    Missing,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl DeviceMetadata {
    /// Returns the synthetic device identifier.
    pub(crate) fn device(&self) -> u64 {
        self.device
    }

    /// Returns the stable inode identifier.
    pub(crate) fn inode(&self) -> u64 {
        self.inode
    }

    /// Returns whether this entry is a directory.
    pub(crate) fn is_directory(&self) -> bool {
        self.is_directory
    }
}

impl DevicePath {
    /// Returns metadata for an existing namespace entry.
    pub(crate) fn metadata(self) -> Option<DeviceMetadata> {
        match self {
            DevicePath::Directory => Some(DeviceMetadata {
                device: DEVICE_NAMESPACE_ID,
                inode: DIRECTORY_INODE,
                is_directory: true,
            }),
            DevicePath::Missing => None,
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Resolves a path in the synthetic device namespace.
pub(crate) fn resolve(path: &str, cwd: &str) -> Result<Option<DevicePath>, Fat32Error> {
    let anchored: String = anchor_path(path, cwd)?;
    validate_components(&anchored)?;
    let normalized: String = normalize_anchored(&anchored);
    Ok(resolve_normalized(&normalized))
}

/// Returns whether routing for a path belongs to the synthetic device namespace.
pub(crate) fn owns(path: &str, cwd: &str) -> Result<bool, Fat32Error> {
    let anchored: String = anchor_path(path, cwd)?;
    if validate_components(&anchored).is_err() {
        return Ok(true);
    }
    let normalized: String = normalize_anchored(&anchored);
    Ok(resolve_normalized(&normalized).is_some())
}

/// Resolves metadata for an existing device-namespace path.
pub(crate) fn metadata(path: &str, cwd: &str) -> Result<Option<DeviceMetadata>, Fat32Error> {
    let anchored: String = anchor_path(path, cwd)?;
    validate_components(&anchored)?;
    let normalized: String = normalize_anchored(&anchored);
    let Some(device_path) = resolve_normalized(&normalized) else {
        return Ok(None);
    };
    Ok(Some(device_path.metadata().ok_or(Fat32Error::NotFound)?))
}

/// Rejects traversal through an unknown namespace entry.
fn validate_components(path: &str) -> Result<(), Fat32Error> {
    let mut components: Vec<&str> = Vec::new();
    for component in path.split('/').filter(|component| !component.is_empty()) {
        if components.len() >= 2 && components[0] == "dev" {
            return Err(Fat32Error::NotFound);
        }
        match component {
            "." => {},
            ".." => {
                components.pop();
            },
            component => components.push(component),
        }
    }
    Ok(())
}

/// Classifies a normalized, absolute path.
fn resolve_normalized(path: &str) -> Option<DevicePath> {
    match path {
        "/dev" => Some(DevicePath::Directory),
        path if path.starts_with("/dev/") => Some(DevicePath::Missing),
        _ => None,
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_namespace_paths() {
        assert_eq!(resolve("/dev", "/"), Ok(Some(DevicePath::Directory)));
        assert_eq!(resolve("/dev/unknown", "/"), Ok(Some(DevicePath::Missing)));
        assert_eq!(resolve("/dev/unknown/child", "/"), Err(Fat32Error::NotFound));
    }

    #[test]
    fn routing_ownership_is_distinct_from_validity() {
        assert!(owns("/dev/unknown/child", "/").expect("valid path"));
        assert!(!owns("/dev/../tmp", "/").expect("valid path"));
    }

    #[test]
    fn normalizes_paths_before_resolving() {
        assert_eq!(resolve("/dev/", "/"), Ok(Some(DevicePath::Directory)));
        assert_eq!(resolve("dev", "/"), Ok(Some(DevicePath::Directory)));
        assert_eq!(resolve("/dev/../tmp", "/"), Ok(None));
    }

    #[test]
    fn ignores_similar_prefixes() {
        assert_eq!(resolve("/", "/"), Ok(None));
        assert_eq!(resolve("/device", "/"), Ok(None));
        assert_eq!(resolve("/devil/null", "/"), Ok(None));
    }
}
