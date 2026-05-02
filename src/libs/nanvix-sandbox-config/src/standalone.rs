// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Standalone deployment mode configuration.
//!
//! This module provides the configuration structure for standalone mode, where the HTTP client
//! directly drives User VM instances without going through a sandbox cache, system VM,
//! control-plane, or gateway.

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Configuration for standalone mode.
///
/// Holds the minimal set of paths required to launch a User VM without the full sandbox
/// cache infrastructure.
///
#[derive(Clone)]
pub struct StandaloneConfig {
    /// Path to the guest kernel binary.
    kernel_binary_path: String,
    /// Optional path to a RAM filesystem image exposed to the guest.
    ramfs_filename: Option<String>,
    /// Optional file path for capturing guest stderr output.
    console_file: Option<String>,
    /// Optional snapshot path for restoring VM state instead of cold-booting.
    snapshot_path: Option<String>,
    /// Optional host directory to mount on the guest.
    mount_directory: Option<String>,
    /// Optional GDB server port for debugging the guest.
    #[cfg(feature = "gdb")]
    gdb_port: Option<u16>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl StandaloneConfig {
    ///
    /// # Description
    ///
    /// Creates a new standalone configuration.
    ///
    /// # Parameters
    ///
    /// - `kernel_binary_path`: Path to the guest kernel binary.
    /// - `ramfs_filename`: Optional path to a RAM filesystem image.
    /// - `console_file`: Optional file path for guest stderr capture.
    /// - `snapshot_path`: Optional snapshot path for restoring VM state instead of cold-booting.
    /// - `mount_directory`: Optional host directory to mount on the guest.
    /// - `gdb_port`: Optional GDB server port.
    ///
    pub fn new(
        kernel_binary_path: String,
        ramfs_filename: Option<String>,
        console_file: Option<String>,
        snapshot_path: Option<String>,
        mount_directory: Option<String>,
        #[cfg(feature = "gdb")] gdb_port: Option<u16>,
    ) -> Self {
        Self {
            kernel_binary_path,
            ramfs_filename,
            console_file,
            snapshot_path,
            mount_directory,
            #[cfg(feature = "gdb")]
            gdb_port,
        }
    }

    /// Returns the path to the guest kernel binary.
    pub fn kernel_binary_path(&self) -> &str {
        &self.kernel_binary_path
    }

    /// Returns the optional RAM filesystem image filename.
    pub fn ramfs_filename(&self) -> Option<&str> {
        self.ramfs_filename.as_deref()
    }

    /// Returns the optional file path for guest stderr capture.
    pub fn console_file(&self) -> Option<&str> {
        self.console_file.as_deref()
    }

    /// Returns the optional snapshot path for restoring VM state.
    pub fn snapshot_path(&self) -> Option<&str> {
        self.snapshot_path.as_deref()
    }

    /// Returns the optional host directory to mount on the guest.
    pub fn mount_directory(&self) -> Option<&str> {
        self.mount_directory.as_deref()
    }

    /// Returns the optional GDB server port.
    #[cfg(feature = "gdb")]
    pub fn gdb_port(&self) -> Option<u16> {
        self.gdb_port
    }
}

///
/// # Description
///
/// Configuration for the terminal in standalone mode.
///
/// Holds the minimal set of paths required to launch a User VM directly.
///
#[derive(Clone)]
pub struct TerminalConfig {
    /// Path to the guest kernel binary.
    kernel_binary_path: String,
    /// Optional path to a RAM filesystem image exposed to the guest.
    ramfs_filename: Option<String>,
    /// Optional file path for capturing guest stderr output.
    console_file: Option<String>,
    /// Optional snapshot path for restoring VM state instead of cold-booting.
    snapshot_path: Option<String>,
    /// Optional host directory to mount on the guest.
    mount_directory: Option<String>,
    /// Optional GDB server port for debugging the guest.
    #[cfg(feature = "gdb")]
    gdb_port: Option<u16>,
}

impl TerminalConfig {
    ///
    /// # Description
    ///
    /// Creates a new terminal configuration.
    ///
    /// # Parameters
    ///
    /// - `kernel_binary_path`: Path to the guest kernel binary.
    /// - `ramfs_filename`: Optional path to a RAM filesystem image.
    /// - `console_file`: Optional file path for guest stderr capture.
    /// - `snapshot_path`: Optional snapshot path for restoring VM state instead of cold-booting.
    /// - `mount_directory`: Optional host directory to mount on the guest.
    /// - `gdb_port`: Optional GDB server port.
    ///
    pub fn new(
        kernel_binary_path: String,
        ramfs_filename: Option<String>,
        console_file: Option<String>,
        snapshot_path: Option<String>,
        mount_directory: Option<String>,
        #[cfg(feature = "gdb")] gdb_port: Option<u16>,
    ) -> Self {
        Self {
            kernel_binary_path,
            ramfs_filename,
            console_file,
            snapshot_path,
            mount_directory,
            #[cfg(feature = "gdb")]
            gdb_port,
        }
    }

    /// Returns the path to the guest kernel binary.
    pub fn kernel_binary_path(&self) -> &str {
        &self.kernel_binary_path
    }

    /// Returns the optional RAM filesystem image filename.
    pub fn ramfs_filename(&self) -> Option<&str> {
        self.ramfs_filename.as_deref()
    }

    /// Returns the optional file path for guest stderr capture.
    pub fn console_file(&self) -> Option<&str> {
        self.console_file.as_deref()
    }

    /// Returns the optional snapshot path for restoring VM state.
    pub fn snapshot_path(&self) -> Option<&str> {
        self.snapshot_path.as_deref()
    }

    /// Returns the optional host directory to mount on the guest.
    pub fn mount_directory(&self) -> Option<&str> {
        self.mount_directory.as_deref()
    }

    /// Returns the optional GDB server port.
    #[cfg(feature = "gdb")]
    pub fn gdb_port(&self) -> Option<u16> {
        self.gdb_port
    }
}
