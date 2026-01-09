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

#[cfg(not(feature = "single-process"))]
use crate::netns::{
    NetnsHandle,
    NetnsInfo,
};
use crate::{
    linuxd::LinuxDaemon,
    InitializedSandbox,
    LinuxDaemonArgs,
    SandboxConfig,
};
use ::anyhow::Result;
#[cfg(not(feature = "single-process"))]
use ::std::marker::PhantomData;
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
/// # Type Parameters
///
/// - `T`: Custom state type for the syscall table. This is passed to system call handlers in
///   single-process mode. Use `()` if no custom state is required.
///
pub struct UninitializedSandbox<T> {
    /// Path to the guest binary file to execute.
    guest_binary_path: String,
    /// Optional command-line arguments for the program.
    program_args: Option<String>,
    /// Optional handle to an existing Linux Daemon instance.
    linuxd: Option<Arc<LinuxDaemon>>,
    /// Optional handle to a network namespace. Only used in L2 deployments.
    #[cfg(not(feature = "single-process"))]
    netns_handle: Option<NetnsHandle>,
    /// Optional control plane listener socket, address, and socket type.
    control_plane_bind_socket_and_info: Option<Arc<Mutex<(SocketListener, String, SocketType)>>>,
    /// Optional sandbox configuration parameters.
    config: Option<SandboxConfig<T>>,
    /// Phantom data to maintain the generic type parameter `T` in the structure.
    /// This is required because `T` is only used in single-process mode for the syscall table.
    #[cfg(not(feature = "single-process"))]
    _phantom: PhantomData<T>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<T: Sync + Send + Default + 'static> UninitializedSandbox<T> {
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
            #[cfg(not(feature = "single-process"))]
            netns_handle: None,
            control_plane_bind_socket_and_info: None,
            config: None,
            #[cfg(not(feature = "single-process"))]
            _phantom: PhantomData,
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
    pub fn with_config(mut self, config: SandboxConfig<T>) -> Self {
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
    /// Adds a network namespace handle to the target uninitialized sandbox.
    ///
    /// # Parameters
    ///
    /// - `netns_handle`: Optional handle to a network namespace.
    ///
    /// # Returns
    ///
    /// The modified uninitialized sandbox with the network namespace handle attached.
    ///
    #[cfg(not(feature = "single-process"))]
    pub fn with_netns_handle(mut self, netns_handle: Option<NetnsHandle>) -> Self {
        self.netns_handle = netns_handle;
        self
    }

    ///
    /// # Description
    ///
    /// Adds a control plane socket and info to the target uninitialized sandbox.
    ///
    /// # Parameters
    ///
    /// - `control_plane_bind_socket_and_info`: Control plane socket listener, address, and socket type.
    ///
    /// # Returns
    ///
    /// The modified uninitialized sandbox with the control plane socket attached.
    ///
    pub fn with_control_plane_bind_socket(
        mut self,
        control_plane_bind_socket_and_info: Arc<Mutex<(SocketListener, String, SocketType)>>,
    ) -> Self {
        self.control_plane_bind_socket_and_info = Some(control_plane_bind_socket_and_info);
        self
    }

    ///
    /// # Description
    ///
    /// Returns the network namespace information for this sandbox.
    ///
    /// # Returns
    ///
    /// Returns the network namespace information if available, or `None` otherwise.
    ///
    #[cfg(not(feature = "single-process"))]
    pub fn netns_info(&self) -> Option<NetnsInfo> {
        if let Some(netns_handle) = &self.netns_handle {
            netns_handle.netns_info().ok()
        } else {
            None
        }
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
    /// On success, returns an initialized sandbox ready to be started.
    ///
    /// # Errors
    ///
    /// This function returns an error if the sandbox configuration is not provided, if the
    /// control plane socket cannot be bound, or if the Linux Daemon fails to spawn.
    ///
    pub async fn initialize(mut self) -> Result<InitializedSandbox<T>> {
        // Get sandbox configuration.
        let config: SandboxConfig<T> = match self.config.take() {
            None => {
                let reason: &str = "sandbox configuration not provided";
                error!("initialize(): {reason}");
                anyhow::bail!(reason);
            },
            Some(config) => config,
        };

        // Get the control-plane listener socket.
        let control_plane_bind_socket_and_info: Arc<Mutex<(SocketListener, String, SocketType)>> =
            match self.control_plane_bind_socket_and_info.take() {
                // Control-plane listener socket not yet initialized.
                None => {
                    // Get control-plane listener socket info.
                    let (control_plane_bind_socket_address, control_plane_bind_socket_type) =
                        match config.control_plane_bind_socket_info() {
                            None => {
                                let reason: &str = "control plane listener socket info not \
                                                    provided and control plane listener socket \
                                                    not initialized";
                                error!("initialize(): {reason}");
                                anyhow::bail!(reason);
                            },
                            Some((addr, stype)) => (addr.clone(), *stype),
                        };

                    let unbound_socket: UnboundSocket =
                        UnboundSocket::new(control_plane_bind_socket_type.to_owned());
                    let control_plane_bind_socket: SocketListener = match unbound_socket
                        .bind(&control_plane_bind_socket_address)
                        .await
                    {
                        Ok(listener) => listener,
                        Err(error) => {
                            let reason: String = format!(
                                    "failed to bind control-plane socket \
                                     (control_plane_bind_socket_address={control_plane_bind_socket_address}, \
                                     error={error:?})"
                                );
                            error!("initialize(): {reason}");
                            anyhow::bail!(reason);
                        },
                    };

                    Arc::new(Mutex::new((
                        control_plane_bind_socket,
                        control_plane_bind_socket_address,
                        control_plane_bind_socket_type,
                    )))
                },
                Some(control_plane_bind_socket_and_info) => control_plane_bind_socket_and_info,
            };

        // Get Linux Daemon.
        let linuxd: Arc<LinuxDaemon> = match self.linuxd.take() {
            // Linux Daemon not yet initialized.
            None => {
                let mut locked_control_plane_bind_socket_and_info: MutexGuard<
                    '_,
                    (SocketListener, String, SocketType),
                > = control_plane_bind_socket_and_info.lock().await;

                // Build Linux Daemon arguments.
                let linuxd_args: LinuxDaemonArgs<T> = {
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

                    // Get L2 snapshot path.
                    let l2_snapshot_path: &str = match config.l2_snapshot_path() {
                        None => {
                            let reason: &str =
                                "L2 snapshot path not provided and linuxd not initialized";
                            error!("initialize(): {reason}");
                            anyhow::bail!(reason);
                        },
                        Some(l2_snapshot_path) => l2_snapshot_path,
                    };

                    LinuxDaemonArgs::new(
                        // We pass linuxd the control plane socket's connect address, which may
                        // depend on the network namespace.
                        (
                            config.control_plane_connect_socket_info().0.clone(),
                            config.control_plane_connect_socket_info().1,
                        ),
                        config.system_vm_socket_info().clone(),
                        config.hwloc(),
                        #[cfg(not(feature = "single-process"))]
                        config.linuxd_binary_path().to_string(),
                        toolchain_binary_directory,
                        config.log_directory().to_string(),
                        tmp_directory,
                        l2,
                        l2_snapshot_path.to_string(),
                        #[cfg(feature = "single-process")]
                        config.syscall_table(),
                    )
                };

                // Spawn Linux Daemon.
                match LinuxDaemon::spawn(
                    &linuxd_args,
                    // Pass a mutable reference to the shared listener socket to accept one
                    // incoming connection from the newly spawned linuxd instance.
                    &mut locked_control_plane_bind_socket_and_info.0,
                    // Share ownership of netns handle with linux daemon process. The netns is
                    // provisioned upstream, if it is not but we are in L2 mode, spawn will fail.
                    #[cfg(not(feature = "single-process"))]
                    self.netns_handle.clone(),
                )
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
            kernel_binary_path: config.kernel_binary_path().to_string(),
            program_args: self.program_args,
            linuxd,
            control_plane_bind_socket_and_info,
            sandbox_config: config,
            // Pass ownership of the network namespace to the initialized sandbox.
            #[cfg(not(feature = "single-process"))]
            netns_handle: self.netns_handle.take(),
            #[cfg(not(feature = "single-process"))]
            _phantom: PhantomData,
        })
    }
}
