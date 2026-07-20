// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Host-level runtime configuration.

use std::path::PathBuf;

use serde::Deserialize;

/// Runtime configuration for the Nanvix shim.
///
/// Specifies paths to host-provided binaries and directories.
/// Loaded from a TOML config file or populated with defaults.
#[derive(Debug, Clone, Deserialize)]
pub struct NanvixRuntimeConfig {
    /// Path to the `nanvixd.elf` kernel binary on the host.
    #[serde(default = "default_kernel_path")]
    pub kernel_path: PathBuf,

    /// Path to the `mkramfs.elf` tool on the host.
    #[serde(default = "default_mkramfs_path")]
    pub mkramfs_path: PathBuf,

    /// Temporary directory for generated ramfs images.
    #[serde(default = "default_temp_dir")]
    pub temp_dir: PathBuf,

    /// Additional arguments passed to nanvixd.
    #[serde(default)]
    pub extra_args: Vec<String>,
}

impl Default for NanvixRuntimeConfig {
    fn default() -> Self {
        Self {
            kernel_path: default_kernel_path(),
            mkramfs_path: default_mkramfs_path(),
            temp_dir: default_temp_dir(),
            extra_args: Vec::new(),
        }
    }
}

fn default_kernel_path() -> PathBuf {
    PathBuf::from("nanvixd.elf")
}

fn default_mkramfs_path() -> PathBuf {
    PathBuf::from("mkramfs.elf")
}

fn default_temp_dir() -> PathBuf {
    std::env::temp_dir()
}

impl NanvixRuntimeConfig {
    /// Load configuration from a TOML file, falling back to defaults for missing fields.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load from the default config path, or return defaults if the file doesn't exist.
    pub fn load_or_default() -> Self {
        let candidates = [
            PathBuf::from("/etc/nanvix/shim-config.toml"),
            PathBuf::from("shim-config.toml"),
        ];
        for path in &candidates {
            if path.exists() {
                if let Ok(config) = Self::load(path) {
                    return config;
                }
            }
        }
        Self::default()
    }
}
