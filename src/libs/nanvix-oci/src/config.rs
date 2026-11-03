// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Nanvix image configuration parsed from OCI spec annotations.

use std::collections::HashMap;

use crate::annotations;

/// Configuration extracted from `com.nanvix.*` OCI annotations.
#[derive(Debug, Clone)]
pub struct NanvixImageConfig {
    /// Path to the application binary within the image (required).
    pub initrd_path: String,
    /// Arguments passed to the application.
    pub initrd_args: Vec<String>,
    /// Environment variables for the application.
    pub initrd_env: Vec<String>,
    /// Path to the ramfs directory within the image. If `None`, no ramfs is used.
    pub ramfs_root: Option<String>,
    /// Target architecture.
    pub arch: String,
    /// Optional Nanvix version hint.
    pub version: Option<String>,
}

impl NanvixImageConfig {
    /// Parse from a map of OCI annotations.
    ///
    /// Returns `None` if the image is not a Nanvix image (i.e., `com.nanvix.os` is absent
    /// or not equal to `"nanvix"`).
    pub fn from_annotations(labels: &HashMap<String, String>) -> Option<Self> {
        let os = labels.get(annotations::OS)?;
        if os != "nanvix" {
            return None;
        }

        let initrd_path = labels.get(annotations::INITRD_PATH)?.clone();
        if initrd_path.is_empty() {
            return None;
        }

        let initrd_args = labels
            .get(annotations::INITRD_ARGS)
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_default();

        let initrd_env = labels
            .get(annotations::INITRD_ENV)
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_default();

        let ramfs_root = labels
            .get(annotations::RAMFS_ROOT)
            .filter(|s| !s.is_empty())
            .cloned();

        let arch = labels
            .get(annotations::ARCH)
            .cloned()
            .unwrap_or_else(|| "x86".to_string());

        let version = labels
            .get(annotations::VERSION)
            .filter(|s| !s.is_empty())
            .cloned();

        Some(NanvixImageConfig {
            initrd_path,
            initrd_args,
            initrd_env,
            ramfs_root,
            arch,
            version,
        })
    }

    /// Parse from an OCI runtime spec.
    ///
    /// Reads annotations from `spec.annotations()` or falls back to
    /// `spec.process().env()` labels convention.
    pub fn from_oci_spec(spec: &oci_spec::runtime::Spec) -> Option<Self> {
        let annotations = spec.annotations().as_ref()?;
        Self::from_annotations(annotations)
    }
}

#[cfg(test)]
mod tests;
