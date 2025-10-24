// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Uninitialized sandbox state management.
//!
//! This module defines the `UninitializedSandbox` structure and its methods for creating
//! and configuring a sandbox before initialization. It provides a builder pattern for
//! setting up Linux Daemon instances, control plane sockets, and sandbox configurations.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sandbox::{
    linuxd::LinuxDaemon,
    InitializedSandbox,
    LinuxDaemonArgs,
    SandboxConfig,
};
use ::anyhow::Result;
use ::std::sync::Arc;
use ::syscomm::{
    SocketListener,
    SocketType,
    UnboundSocket,
};
use ::syslog::error;
use ::tokio::sync::{
    Mutex,
    MutexGuard,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// An uninitialized sandbox.
///
/// This structure represents a sandbox in its initial state before initialization. It uses
/// a builder pattern to accumulate configuration and resources (Linux Daemon, control plane
/// socket, configuration) before transitioning to an initialized state.
///
pub struct UninitializedSandbox {
    /// Path to the guest binary file to execute.
    guest_binary_path: String,
    /// Optional command-line arguments for the program.
    program_args: Option<String>,
    /// Optional handle to an existing Linux Daemon instance.
    linuxd: Option<Arc<LinuxDaemon>>,
    /// Optional control plane listener socket, address, and socket type.
    control_plane_socket_and_info: Option<Arc<Mutex<(SocketListener, String, SocketType)>>>,
    /// Optional sandbox configuration parameters.
    config: Option<SandboxConfig>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl UninitializedSandbox {
    ///
    /// # Description
    ///
    /// Creates a new instance of an uninitialized sandbox.
    ///
    /// # Parameters
    ///
    /// - `guest_binary_path`: Path to the guest binary file to execute.
    /// - `program_args`: Optional command-line arguments for the program.
    ///
    /// # Returns
    ///
    /// A new instance of an uninitialized sandbox.
    ///
    pub fn new(guest_binary_path: &str, program_args: Option<String>) -> Self {
        UninitializedSandbox {
            guest_binary_path: guest_binary_path.to_string(),
            program_args,
            linuxd: None,
            control_plane_socket_and_info: None,
            config: None,
        }
    }

    ///
    /// # Description
    ///
    /// Adds a sandbox configuration to the target uninitialized sandbox.
    ///
    /// # Parameters
    ///
    /// - `config`: Sandbox configuration.
    ///
    /// # Returns
    ///
    /// This function returns the modified uninitialized sandbox.
    ///
    pub fn with_config(mut self, config: SandboxConfig) -> Self {
        self.config = Some(config);
        self
    }

    ///
    /// # Description
    ///
    /// Adds a Linux Daemon instance to the target uninitialized sandbox.
    ///
    /// # Parameters
    ///
    /// - `linuxd`: Shared handle to an existing Linux Daemon instance.
    ///
    /// # Returns
    ///
    /// The modified uninitialized sandbox with the Linux Daemon attached.
    ///
    pub fn with_linuxd(mut self, linuxd: Arc<LinuxDaemon>) -> Self {
        self.linuxd = Some(linuxd);
        self
    }
    ///
    /// # Description
    ///
    /// Adds a control plane socket and info to the target uninitialized sandbox.
    ///
    /// # Parameters
    ///
    /// - `control_plane_socket_and_info`: Control plane socket listener, address, and socket type.
    ///
    /// # Returns
    ///
    /// The modified uninitialized sandbox with the control plane socket attached.
    ///
    pub fn with_control_plane_socket(
        mut self,
        control_plane_socket_and_info: Arc<Mutex<(SocketListener, String, SocketType)>>,
    ) -> Self {
        self.control_plane_socket_and_info = Some(control_plane_socket_and_info);
        self
    }

    ///
    /// # Description
    ///
    /// Initializes the sandbox by setting up the control plane socket and spawning the Linux
    /// Daemon if not already provided. This transitions the sandbox from uninitialized to
    /// initialized state.
    ///
    /// # Returns
    ///
    /// On success, returns an initialized sandbox ready to be started. On failure, returns
    /// an error describing what went wrong during initialization.
    ///
    pub async fn initialize(mut self) -> Result<InitializedSandbox> {
        // Get sandbox configuration.
        let config: SandboxConfig = match self.config.take() {
            None => {
                let reason: &str = "sandbox configuration not provided";
                error!("initialize(): {reason}");
                anyhow::bail!(reason);
            },
            Some(config) => config,
        };

        // Get control-plane socket.
        let control_plane_socket_and_info: Arc<Mutex<(SocketListener, String, SocketType)>> =
            match self.control_plane_socket_and_info.take() {
                // Control-plane socket not yet initialized.
                None => {
                    // Get control-plane socket info.
                    let (control_plane_socket_address, control_plane_socket_type) =
                        match config.control_plane_socket_info() {
                            None => {
                                let reason: &str = "control plane socket info not provided and \
                                                    control plane socket not initialized";
                                error!("initialize(): {reason}");
                                anyhow::bail!(reason);
                            },
                            Some((addr, stype)) => (addr.clone(), *stype),
                        };

                    let unbound_socket: UnboundSocket =
                        UnboundSocket::new(control_plane_socket_type.to_owned());
                    let control_plane_socket: SocketListener =
                        match unbound_socket.bind(&control_plane_socket_address).await {
                            Ok(listener) => listener,
                            Err(error) => {
                                let reason: String = format!(
                                    "failed to bind control-plane socket \
                                     (control_plane_socket_address={control_plane_socket_address}, \
                                     error={error:?})"
                                );
                                error!("initialize(): {reason}");
                                anyhow::bail!(reason);
                            },
                        };

                    Arc::new(Mutex::new((
                        control_plane_socket,
                        control_plane_socket_address,
                        control_plane_socket_type,
                    )))
                },
                Some(control_plane_socket_and_info) => control_plane_socket_and_info,
            };

        // Get Linux Daemon.
        let linuxd: Arc<LinuxDaemon> = match self.linuxd.take() {
            // Linux Daemon not yet initialized.
            None => {
                let mut locked_control_plane_socket_and_info: MutexGuard<
                    '_,
                    (SocketListener, String, SocketType),
                > = control_plane_socket_and_info.lock().await;

                // Build Linux Daemon arguments.
                let linuxd_args: LinuxDaemonArgs = {
                    // Get toolchain binary directory.
                    let toolchain_binary_directory: String =
                        match config.toolchain_binary_directory() {
                            None => {
                                let reason: &str = "toolchain binary directory not provided and \
                                                    linuxd not initialized";
                                error!("initialize(): {reason}");
                                anyhow::bail!(reason);
                            },
                            Some(path) => path.to_string(),
                        };

                    // Get temporary directory.
                    let tmp_directory: String = match config.tmp_directory() {
                        None => {
                            let reason: &str =
                                "temporary directory not provided and linuxd not initialized";
                            error!("initialize(): {reason}");
                            anyhow::bail!(reason);
                        },
                        Some(path) => path.to_string(),
                    };

                    // Get L2 flag.
                    let l2: bool = match config.l2() {
                        None => {
                            let reason: &str = "L2 flag not provided and linuxd not initialized";
                            error!("initialize(): {reason}");
                            anyhow::bail!(reason);
                        },
                        Some(l2) => l2,
                    };

                    LinuxDaemonArgs::new(
                        (
                            locked_control_plane_socket_and_info.1.clone(),
                            locked_control_plane_socket_and_info.2,
                        ),
                        config.system_vm_socket_info().clone(),
                        config.hwloc(),
                        config.binary_directory().to_string(),
                        toolchain_binary_directory,
                        config.log_directory().to_string(),
                        tmp_directory,
                        l2,
                        config.syscall_table(),
                    )
                };

                // Spawn Linux Daemon.
                match LinuxDaemon::spawn(&linuxd_args, &mut locked_control_plane_socket_and_info.0)
                    .await
                {
                    Ok(linuxd) => Arc::new(linuxd),
                    Err(error) => {
                        let reason: String = format!("failed to spawn linuxd (error={error:?})");
                        error!("initialize(): {reason}");
                        anyhow::bail!(reason);
                    },
                }
            },
            Some(linuxd) => linuxd,
        };

        Ok(InitializedSandbox {
            guest_binary_path: self.guest_binary_path,
            program_args: self.program_args,
            linuxd,
            control_plane_socket_and_info,
            sandbox_config: config,
        })
    }
}
