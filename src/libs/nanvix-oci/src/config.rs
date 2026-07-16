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
mod tests {
    use super::*;

    fn make_labels(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn parse_full_annotations() {
        let labels = make_labels(&[
            (annotations::OS, "nanvix"),
            (annotations::ARCH, "x86"),
            (annotations::INITRD_PATH, "/initrd/app.elf"),
            (annotations::INITRD_ARGS, "--verbose /config.txt"),
            (annotations::RAMFS_ROOT, "/ramfs"),
            (annotations::VERSION, "0.12.166"),
        ]);

        let config = NanvixImageConfig::from_annotations(&labels).unwrap();
        assert_eq!(config.initrd_path, "/initrd/app.elf");
        assert_eq!(config.initrd_args, vec!["--verbose", "/config.txt"]);
        assert_eq!(config.ramfs_root, Some("/ramfs".to_string()));
        assert_eq!(config.arch, "x86");
        assert_eq!(config.version, Some("0.12.166".to_string()));
    }

    #[test]
    fn parse_minimal_annotations() {
        let labels = make_labels(&[
            (annotations::OS, "nanvix"),
            (annotations::INITRD_PATH, "/initrd/hello.elf"),
        ]);

        let config = NanvixImageConfig::from_annotations(&labels).unwrap();
        assert_eq!(config.initrd_path, "/initrd/hello.elf");
        assert!(config.initrd_args.is_empty());
        assert!(config.ramfs_root.is_none());
        assert_eq!(config.arch, "x86");
        assert!(config.version.is_none());
    }

    #[test]
    fn not_nanvix_image() {
        let labels = make_labels(&[("other.label", "value")]);
        assert!(NanvixImageConfig::from_annotations(&labels).is_none());
    }

    #[test]
    fn wrong_os() {
        let labels = make_labels(&[
            (annotations::OS, "linux"),
            (annotations::INITRD_PATH, "/initrd/app.elf"),
        ]);
        assert!(NanvixImageConfig::from_annotations(&labels).is_none());
    }

    #[test]
    fn missing_initrd_path() {
        let labels = make_labels(&[(annotations::OS, "nanvix")]);
        assert!(NanvixImageConfig::from_annotations(&labels).is_none());
    }

    #[test]
    fn empty_ramfs_root_treated_as_none() {
        let labels = make_labels(&[
            (annotations::OS, "nanvix"),
            (annotations::INITRD_PATH, "/initrd/app.elf"),
            (annotations::RAMFS_ROOT, ""),
        ]);

        let config = NanvixImageConfig::from_annotations(&labels).unwrap();
        assert!(config.ramfs_root.is_none());
    }
}
