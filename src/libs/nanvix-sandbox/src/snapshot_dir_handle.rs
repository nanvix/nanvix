// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Snapshot directory allocation and management.
//!
//! This module provides an RAII handle for the per-instance snapshot directory used to restore
//! an L2 system VM from a shared cloud-hypervisor snapshot.
//!
//! The main idea behind this module is to create symbolic links of the heavyweight components of a
//! snapshot, like the memory ranges, but support modifying the configuration JSON file to tweak
//! deployment characteristics between time-of-snapshot and time-of-restore.

//==================================================================================================
// Imports
//==================================================================================================

use crate::config::get_clh_snapshot_path;
use ::anyhow::Result;
use ::log::{
    error,
    trace,
    warn,
};
use ::serde_json::Value;
use ::std::{
    fs,
    io::ErrorKind,
    path::{
        Path,
        PathBuf,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// RAII handle to a per-instance snapshot directory used to restore the L2 system VM.
///
/// The handle materializes a private directory containing:
/// - `state.json`: a symbolic link to the shared snapshot file.
/// - `memory-ranges`: a symbolic link to the shared snapshot file.
/// - `config.json`: a rewritten copy of the shared snapshot configuration with a per-instance
///   console file path.
///
pub struct SnapshotDirHandle {
    path: PathBuf,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SnapshotDirHandle {
    const STATE_FILE: &'static str = "state.json";
    const MEMORY_RANGES_FILE: &'static str = "memory-ranges";
    const CONFIG_FILE: &'static str = "config.json";

    ///
    /// # Description
    ///
    /// Creates a per-instance snapshot directory and populates it from the shared snapshot.
    ///
    /// # Parameters
    ///
    /// - `path`: Destination directory for the per-instance snapshot files.
    /// - `snapshot_source_dir`: Source directory containing the shared snapshot files.
    /// - `console_file`: Console file path to embed in the rewritten `config.json`.
    ///
    /// # Returns
    ///
    /// A RAII handle to the created snapshot directory.
    ///
    pub fn new<P: AsRef<Path>, Q: AsRef<Path>, R: AsRef<Path>>(
        path: P,
        snapshot_source_dir: Q,
        console_file: R,
    ) -> Result<Self> {
        let path: PathBuf = path.as_ref().to_path_buf();
        let snapshot_source_dir: PathBuf =
            PathBuf::from(get_clh_snapshot_path(&snapshot_source_dir.as_ref().to_string_lossy())?);
        let console_file: String = console_file.as_ref().to_string_lossy().into_owned();

        if path.exists() {
            fs::remove_dir_all(&path).map_err(|error| {
                let reason: String =
                    format!("failed to reset snapshot directory (path={path:?}, error={error:?})");
                error!("new(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        fs::create_dir_all(&path).map_err(|error| {
            let reason: String =
                format!("failed to create snapshot directory (path={path:?}, error={error:?})");
            error!("new(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        Self::create_symlink(
            snapshot_source_dir.join(Self::STATE_FILE),
            path.join(Self::STATE_FILE),
        )?;
        Self::create_symlink(
            snapshot_source_dir.join(Self::MEMORY_RANGES_FILE),
            path.join(Self::MEMORY_RANGES_FILE),
        )?;
        Self::write_config_file(
            &snapshot_source_dir.join(Self::CONFIG_FILE),
            &path.join(Self::CONFIG_FILE),
            &console_file,
        )?;

        Ok(Self { path })
    }

    ///
    /// # Description
    ///
    /// Returns the path to the per-instance snapshot directory.
    ///
    /// # Returns
    ///
    /// The path to the per-instance snapshot directory.
    ///
    pub fn path(&self) -> &Path {
        &self.path
    }

    ///
    /// # Description
    ///
    /// Helper method to create a symbolic link.
    ///
    /// # Arguments
    ///
    /// - `source`: source of the symbolic link.
    /// - `destination`: destination of the symbolic link.
    ///
    fn create_symlink(source: PathBuf, destination: PathBuf) -> Result<()> {
        ::std::os::unix::fs::symlink(&source, &destination).map_err(|error| {
            let reason: String = format!(
                "failed to create snapshot symlink (source={source:?}, \
                 destination={destination:?}, error={error:?})"
            );
            error!("create_symlink(): {reason}");
            anyhow::anyhow!(reason)
        })
    }

    ///
    /// # Description
    ///
    /// Helper method to create a per-L2 VM config.json snapshot file with a modified log file.
    /// Right now we only modify the console-file field, but in the future this method could be
    /// used to modify other fields, e.g. the TAP device name, were it to proof necessary.
    ///
    /// # Arguments
    ///
    /// - `source`: path of the original configuration file, the source of truth.
    /// - `destination`: path of the new configuration file.
    /// - `console_file`: value to overwrite in the config.json file.
    ///
    fn write_config_file(source: &Path, destination: &Path, console_file: &str) -> Result<()> {
        let config_contents: String = fs::read_to_string(source).map_err(|error| {
            let reason: String =
                format!("failed to read snapshot config (path={source:?}, error={error:?})");
            error!("write_config_file(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        let mut config: Value = ::serde_json::from_str(&config_contents).map_err(|error| {
            let reason: String = format!(
                "failed to parse snapshot config as JSON (path={source:?}, error={error:?})"
            );
            error!("write_config_file(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        // Update console.file value.
        config["console"]["file"] = Value::String(console_file.to_string());

        let rendered: Vec<u8> = ::serde_json::to_vec_pretty(&config).map_err(|error| {
            let reason: String = format!(
                "failed to render snapshot config as JSON (path={destination:?}, error={error:?})"
            );
            error!("write_config_file(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        fs::write(destination, rendered).map_err(|error| {
            let reason: String =
                format!("failed to write snapshot config (path={destination:?}, error={error:?})");
            error!("write_config_file(): {reason}");
            anyhow::anyhow!(reason)
        })
    }
}

impl Drop for SnapshotDirHandle {
    ///
    /// # Description
    ///
    /// Drops the snapshot directory handle, removing the backing directory recursively.
    ///
    fn drop(&mut self) {
        trace!("drop(): removing snapshot directory (path={:?})", self.path);
        if let Err(error) = fs::remove_dir_all(&self.path) {
            if error.kind() != ErrorKind::NotFound {
                warn!(
                    "drop(): failed to remove snapshot directory (path={:?}, error={error:?})",
                    self.path
                );
            }
        }
    }
}
