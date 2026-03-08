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

#[cfg(not(feature = "standalone"))]
use crate::linuxd::LinuxDaemon;
#[cfg(not(any(feature = "single-process", feature = "standalone")))]
use crate::netns::{
    NetnsHandle,
    NetnsInfo,
};
#[cfg(not(feature = "standalone"))]
use crate::LinuxDaemonArgs;
#[cfg(not(any(feature = "single-process", feature = "standalone")))]
use crate::SnapshotDirHandle;
use crate::{
    InitializedSandbox,
    SandboxConfig,
};
use ::anyhow::Result;
use ::log::error;
#[cfg(not(any(feature = "single-process", feature = "standalone")))]
use ::std::marker::PhantomData;
use ::std::sync::Arc;
use ::syscomm::{
    SocketListener,
    SocketType,
};
use ::tokio::sync::Mutex;
#[cfg(not(feature = "standalone"))]
use ::tokio::sync::MutexGuard;

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
    /// Optional RAM filesystem image exposed to the guest program.
    ramfs_filename: Option<String>,
    /// Optional handle to an existing Linux Daemon instance.
    #[cfg(not(feature = "standalone"))]
    linuxd: Option<Arc<LinuxDaemon>>,
    /// Optional handle to a network namespace. Only used in L2 deployments.
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    netns_handle: Option<NetnsHandle>,
    /// Optional handle to the per-instance snapshot directory. Only used in L2 deployments.
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    snapshot_dir_handle: Option<SnapshotDirHandle>,
    /// Optional control plane listener socket, address, and socket type.
    control_plane_bind_socket_and_info: Option<Arc<Mutex<(SocketListener, String, SocketType)>>>,
    /// Optional sandbox configuration parameters.
    config: Option<SandboxConfig<T>>,
    /// Phantom data to maintain the generic type parameter `T` in the structure.
    /// This is required because `T` is only used in single-process mode for the syscall table.
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
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
    /// - `ramfs_filename`: Optional RAM filesystem image filename to expose to the guest.
    /// - `control_plane_bind_socket_and_info`: Shared control plane socket listener, address, and
    ///   socket type.
    ///
    /// # Returns
    ///
    /// A new instance of an uninitialized sandbox.
    ///
    pub fn new(
        guest_binary_path: &str,
        program_args: Option<String>,
        ramfs_filename: Option<String>,
        control_plane_bind_socket_and_info: Arc<Mutex<(SocketListener, String, SocketType)>>,
    ) -> Self {
        UninitializedSandbox {
            guest_binary_path: guest_binary_path.to_string(),
            program_args,
            ramfs_filename,
            #[cfg(not(feature = "standalone"))]
            linuxd: None,
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
            netns_handle: None,
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
            snapshot_dir_handle: None,
            control_plane_bind_socket_and_info: Some(control_plane_bind_socket_and_info),
            config: None,
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
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
    #[cfg(not(feature = "standalone"))]
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
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    pub fn with_netns_handle(mut self, netns_handle: Option<NetnsHandle>) -> Self {
        self.netns_handle = netns_handle;
        self
    }

    ///
    /// # Description
    ///
    /// Adds a snapshot directory handle to the target uninitialized sandbox.
    ///
    /// # Parameters
    ///
    /// - `snapshot_dir_handle`: Optional handle to the per-instance snapshot directory.
    ///
    /// # Returns
    ///
    /// The modified uninitialized sandbox with the snapshot directory handle attached.
    ///
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    pub fn with_snapshot_dir_handle(
        mut self,
        snapshot_dir_handle: Option<SnapshotDirHandle>,
    ) -> Self {
        self.snapshot_dir_handle = snapshot_dir_handle;
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
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
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
    /// Initializes the sandbox by spawning the Linux Daemon if not already provided. This
    /// transitions the sandbox from uninitialized to initialized state.
    ///
    /// The control plane socket must be provided via `new()` before calling this method. The
    /// socket is expected to be pre-bound by the caller (typically `SandboxCache::new()`).
    ///
    /// # Returns
    ///
    /// On success, returns an initialized sandbox ready to be started.
    ///
    /// # Errors
    ///
    /// This function returns an error if the sandbox configuration is not provided, if the
    /// control plane socket was not provided, or if the Linux Daemon fails to spawn.
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

        // Get the control-plane listener socket (must be provided via new()).
        let control_plane_bind_socket_and_info: Arc<Mutex<(SocketListener, String, SocketType)>> =
            match self.control_plane_bind_socket_and_info.take() {
                Some(control_plane_bind_socket_and_info) => control_plane_bind_socket_and_info,
                None => {
                    let reason: &str = "control plane listener socket not provided via new()";
                    error!("initialize(): {reason}");
                    anyhow::bail!(reason);
                },
            };

        // Get Linux Daemon.
        #[cfg(not(feature = "standalone"))]
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

                    LinuxDaemonArgs::new(
                        config.tenant_id(),
                        // We pass linuxd the control plane socket's connect address, which may
                        // depend on the network namespace.
                        (
                            config.control_plane_connect_socket_info().0.clone(),
                            config.control_plane_connect_socket_info().1,
                        ),
                        config.system_vm_socket_info().clone(),
                        config.hwloc(),
                        #[cfg(not(any(feature = "single-process", feature = "standalone")))]
                        config.linuxd_binary_path().to_string(),
                        toolchain_binary_directory,
                        config.log_directory().to_string(),
                        tmp_directory,
                        l2,
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
                    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
                    self.netns_handle.clone(),
                    // Pass ownership of the snapshot dir handle to the linuxd instance.
                    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
                    self.snapshot_dir_handle.take(),
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
            ramfs_filename: self.ramfs_filename,
            #[cfg(not(feature = "standalone"))]
            linuxd,
            control_plane_bind_socket_and_info,
            sandbox_config: config,
            // Pass ownership of the network namespace to the initialized sandbox.
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
            netns_handle: self.netns_handle.take(),
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
            _phantom: PhantomData,
        })
    }
}
