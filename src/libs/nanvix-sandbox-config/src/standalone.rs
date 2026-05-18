// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Standalone deployment mode configuration.
//!
//! This module provides the configuration structure for standalone mode, where the HTTP client
//! directly drives User VM instances without going through a sandbox cache, system VM,
//! control-plane, or gateway.

//==================================================================================================
// Imports
//==================================================================================================

use crate::NetworkingMode;

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
    /// Optional kernel arguments written to guest control registers.
    kernel_args: Option<String>,
    /// Networking mode (disabled or enabled).
    networking_mode: NetworkingMode,
    /// Optional GDB server port for debugging the guest.
    #[cfg(feature = "gdb")]
    gdb_port: Option<u16>,
    /// Optional path at which to expose the guest **application stdio**
    /// endpoint (container fd 1 / fd 2 — IKC `WriteRequest` data).
    ///
    /// Cross-platform:
    /// - Unix: Unix-domain socket path, e.g. `<state>/container-io.sock`.
    /// - Windows: named pipe path, e.g. `\\.\pipe\nanvix-container-io-<id>`.
    ///
    /// When set, nanvix-http's standalone server binds (or creates) this
    /// endpoint and accepts a single client (the containerd shim). The
    /// client receives guest stdout/stderr and may write guest stdin.
    ///
    /// When unset, nanvix-http falls back to its earlier behavior: on Unix
    /// a temp UDS, on Windows a host file sink — kept so nanvix-bench and
    /// nanvix-terminal continue to work without setting the flag.
    container_io_endpoint: Option<String>,
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
    /// - `kernel_args`: Optional kernel arguments written to guest control registers.
    /// - `networking_mode`: Networking mode for host networking.
    /// - `gdb_port`: Optional GDB server port.
    ///
    pub fn new(
        kernel_binary_path: String,
        ramfs_filename: Option<String>,
        console_file: Option<String>,
        snapshot_path: Option<String>,
        mount_directory: Option<String>,
        kernel_args: Option<String>,
        networking_mode: NetworkingMode,
        #[cfg(feature = "gdb")] gdb_port: Option<u16>,
        container_io_endpoint: Option<String>,
    ) -> Self {
        Self {
            kernel_binary_path,
            ramfs_filename,
            console_file,
            snapshot_path,
            mount_directory,
            kernel_args,
            networking_mode,
            #[cfg(feature = "gdb")]
            gdb_port,
            container_io_endpoint,
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

    /// Returns the optional kernel arguments string.
    pub fn kernel_args(&self) -> Option<&str> {
        self.kernel_args.as_deref()
    }

    /// Returns the networking mode.
    pub fn networking_mode(&self) -> NetworkingMode {
        self.networking_mode
    }

    /// Returns the optional GDB server port.
    #[cfg(feature = "gdb")]
    pub fn gdb_port(&self) -> Option<u16> {
        self.gdb_port
    }

    /// Returns the optional container application stdio endpoint path
    /// (UDS on Unix, named pipe on Windows). See the field doc on
    /// [`StandaloneConfig`] for details.
    pub fn container_io_endpoint(&self) -> Option<&str> {
        self.container_io_endpoint.as_deref()
    }
}
