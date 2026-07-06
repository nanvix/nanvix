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

use crate::{
    HostFilter,
    NetworkdEndpoint,
    NetworkingMode,
};

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
    /// Host egress filter applied to guest `connect()` destinations. Only
    /// meaningful when `networking_mode` is enabled.
    host_filter: HostFilter,
    /// Optional decoupled `networkd` endpoint. When set (and networking is
    /// enabled), the user VM forwards socket system calls to this external
    /// `networkd` process instead of running the network daemon in-process.
    networkd_endpoint: Option<NetworkdEndpoint>,
    /// Optional GDB server port for debugging the guest.
    #[cfg(feature = "gdb")]
    gdb_port: Option<u16>,
    /// Optional path at which standalone mode should expose the gateway
    /// endpoint where a host-side consumer (typically the containerd
    /// shim) exchanges the guest's stdin/stdout.
    ///
    /// - Unix: Unix-domain socket path, e.g. `/tmp/nvx-standalone-gw-<pid>.sock`.
    /// - Windows: named pipe path, e.g. `\\.\pipe\nanvix-standalone-gw-<pid>`.
    ///
    /// When unset, standalone mode falls back to a per-process default
    /// path so legacy consumers (`nanvix-bench`, `nanvix-terminal`)
    /// continue to work without any flag.
    gateway_sockaddr: Option<String>,
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
    /// - `host_filter`: Host egress filter applied to guest connections.
    /// - `networkd_endpoint`: Optional decoupled `networkd` endpoint. When set, socket system
    ///   calls are forwarded to an external `networkd` process instead of an in-process daemon.
    /// - `gdb_port`: Optional GDB server port.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kernel_binary_path: String,
        ramfs_filename: Option<String>,
        console_file: Option<String>,
        snapshot_path: Option<String>,
        mount_directory: Option<String>,
        kernel_args: Option<String>,
        networking_mode: NetworkingMode,
        host_filter: HostFilter,
        networkd_endpoint: Option<NetworkdEndpoint>,
        #[cfg(feature = "gdb")] gdb_port: Option<u16>,
        gateway_sockaddr: Option<String>,
    ) -> Self {
        Self {
            kernel_binary_path,
            ramfs_filename,
            console_file,
            snapshot_path,
            mount_directory,
            kernel_args,
            networking_mode,
            host_filter,
            networkd_endpoint,
            #[cfg(feature = "gdb")]
            gdb_port,
            gateway_sockaddr,
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

    /// Returns the host egress filter.
    pub fn host_filter(&self) -> HostFilter {
        self.host_filter.clone()
    }

    /// Returns the optional decoupled `networkd` endpoint.
    pub fn networkd_endpoint(&self) -> Option<NetworkdEndpoint> {
        self.networkd_endpoint.clone()
    }

    /// Returns the optional GDB server port.
    #[cfg(feature = "gdb")]
    pub fn gdb_port(&self) -> Option<u16> {
        self.gdb_port
    }

    /// Returns the optional gateway endpoint path (UDS on Unix, named
    /// pipe on Windows). See the field doc on [`StandaloneConfig`] for
    /// details.
    pub fn gateway_sockaddr(&self) -> Option<&str> {
        self.gateway_sockaddr.as_deref()
    }
}
