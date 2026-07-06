// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Initialized sandbox state management.
//!
//! This module defines the `InitializedSandbox` structure representing a sandbox that has
//! been initialized but not yet started. It includes methods for spawning User VM instances
//! and transitioning to the running state.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "standalone"))]
use crate::ControlPlaneAcceptor;
#[cfg(not(feature = "standalone"))]
use crate::{
    config::{
        CONTROL_PLANE_ACCEPT_TIMEOUT,
        GATEWAY_CONNECT_TIMEOUT,
    },
    linuxd::LinuxDaemon,
    uservm::PendingUserVm,
};
use crate::{
    uservm::UserVm,
    RunningSandbox,
    SandboxConfig,
    SandboxTag,
    UserVmArgs,
};
use ::anyhow::Result;
use ::log::error;
#[cfg(not(any(feature = "single-process", feature = "standalone")))]
use ::std::marker::PhantomData;
#[cfg(not(feature = "standalone"))]
use ::std::sync::Arc;
#[cfg(not(feature = "standalone"))]
use ::syscomm::SocketStream;
use ::syscomm::SocketType;
#[cfg(not(feature = "standalone"))]
use ::tokio::{
    sync::oneshot::Receiver,
    time::timeout,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// An initialized sandbox that is ready to be started.
///
/// This structure represents a sandbox that has completed initialization with a bound control
/// plane socket and a spawned Linux Daemon instance, but has not yet started executing the
/// guest program. It holds all necessary resources to transition to a running state.
///
/// # Type Parameters
///
/// - `T`: Custom state type for the syscall table. This is passed to system call handlers in
///   single-process mode. Must implement `Send + Sync + Default`. Use `()` if no custom state is required.
///
pub struct InitializedSandbox<T: Send + Sync + Default + 'static> {
    /// Path to the guest binary file to execute.
    pub(super) guest_binary_path: String,
    /// Path to the kernel binary.
    pub(super) kernel_binary_path: String,
    /// Optional command-line arguments for the program.
    pub(super) program_args: Option<String>,
    /// Optional RAM filesystem image exposed to the guest program.
    pub(super) ramfs_filename: Option<String>,
    /// Shared handle to the Linux Daemon instance managing this sandbox.
    #[cfg(not(feature = "standalone"))]
    pub(super) linuxd: Arc<LinuxDaemon>,
    /// Shared control-plane acceptor used to route child connections.
    #[cfg(not(feature = "standalone"))]
    pub(super) control_plane_acceptor: Arc<ControlPlaneAcceptor>,
    /// Complete configuration for the sandbox execution environment.
    pub(super) sandbox_config: SandboxConfig<T>,
    /// Phantom data to maintain the generic type parameter `T` in the structure.
    /// This is required because `T` is only used in single-process mode for the syscall table.
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    pub(super) _phantom: PhantomData<T>,
}

impl<T: Send + Sync + Default + 'static> InitializedSandbox<T> {
    ///
    /// # Description
    ///
    /// Starts the sandbox by spawning a User VM instance and waiting for the gateway socket
    /// to become available. This transitions the sandbox from initialized to running state.
    ///
    /// # Parameters
    ///
    /// - `tag`: The sandbox tag containing tenant, program, and application information.
    ///
    /// # Returns
    ///
    /// On success, returns a running sandbox with an active User VM. On failure, returns an
    /// error describing what went wrong during startup.
    ///
    #[cfg_attr(feature = "standalone", allow(unused_variables))]
    pub async fn start(self, tag: SandboxTag) -> Result<RunningSandbox> {
        // Extract gateway socket info parts for later use.
        let gateway_sockaddr: String = self.sandbox_config.gateway_socket_info().0.clone();
        let gateway_socket_type: SocketType = self.sandbox_config.gateway_socket_info().1;
        #[cfg(not(feature = "standalone"))]
        let control_plane_connect_socket_info: (String, SocketType) = self
            .sandbox_config
            .control_plane_connect_socket_info()
            .clone();
        #[cfg(not(feature = "standalone"))]
        let system_vm_socket_info: (String, SocketType) =
            self.sandbox_config.system_vm_socket_info().clone();
        let console_file: Option<String> =
            self.sandbox_config.console_file().map(|s| s.to_string());
        let hwloc: Option<hwloc::HwLoc> = self.sandbox_config.hwloc();
        let log_directory: String = self.sandbox_config.log_directory().to_string();
        let uservm_id: ::user_vm_api::UserVmIdentifier = self.sandbox_config.uservm_id();
        #[cfg(not(any(feature = "single-process", feature = "standalone")))]
        let uservm_binary_path: String = self.sandbox_config.uservm_binary_path().to_string();

        // Extract gateway socket info (consumes the config).
        let gateway_socket_info: (String, SocketType) =
            self.sandbox_config.into_gateway_socket_info();

        // Build User VM arguments.
        let uservm_args: UserVmArgs = UserVmArgs::new(
            #[cfg(not(feature = "standalone"))]
            (control_plane_connect_socket_info.0.clone(), control_plane_connect_socket_info.1),
            #[cfg(not(feature = "standalone"))]
            (gateway_sockaddr.clone(), gateway_socket_type),
            #[cfg(not(feature = "standalone"))]
            system_vm_socket_info,
            self.guest_binary_path.clone(),
            self.program_args.clone(),
            self.ramfs_filename.clone(),
            console_file,
            hwloc,
            self.kernel_binary_path.clone(),
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
            uservm_binary_path,
            log_directory,
            uservm_id,
        );

        // Spawn User VM.
        //
        // In standalone mode, the VM runs without any external connections so there is no
        // need to acquire the control-plane listener or wait for the gateway.
        #[cfg(feature = "standalone")]
        let uservm: UserVm = match UserVm::spawn(&uservm_args).await {
            Ok(uservm) => uservm,
            Err(error) => {
                error!("start(): failed to spawn uservm (error={error:?})");
                return Err(error);
            },
        };

        #[cfg(not(feature = "standalone"))]
        let uservm: UserVm = {
            // Register interest in the user VM we are about to spawn.
            let control_plane_stream_rx: Receiver<SocketStream> = self
                .control_plane_acceptor
                .register_uservm(uservm_id)
                .await?;

            // Spawn the user VM.
            let pending_uservm: PendingUserVm = match UserVm::spawn(&uservm_args).await {
                Ok(uservm) => uservm,
                Err(error) => {
                    self.control_plane_acceptor
                        .unregister_uservm(uservm_id)
                        .await;
                    error!("start(): failed to spawn uservm (error={error:?})");
                    return Err(error);
                },
            };

            // Await for the user VM to send a handshake message.
            let control_plane_stream: SocketStream =
                match timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, control_plane_stream_rx).await {
                    Ok(Ok(stream)) => stream,
                    Ok(Err(error)) => {
                        self.control_plane_acceptor
                            .unregister_uservm(uservm_id)
                            .await;
                        let reason: String = format!(
                            "control-plane acceptor dropped before delivering uservm stream \
                             (uservm_id={uservm_id}, error={error:?})"
                        );
                        error!("start(): {reason}");
                        pending_uservm.abort().await;
                        anyhow::bail!(reason);
                    },
                    Err(error) => {
                        self.control_plane_acceptor
                            .unregister_uservm(uservm_id)
                            .await;
                        let reason: String = format!(
                            "timed-out waiting for uservm control-plane connection \
                             (uservm_id={uservm_id}, error={error:?})"
                        );
                        error!("start(): {reason}");
                        pending_uservm.abort().await;
                        anyhow::bail!(reason);
                    },
                };

            // Upgrade the pending user VM to a full user VM by attaching the received
            // control-plane stream.
            pending_uservm.attach_control_plane(control_plane_stream)
        };

        // Wait for linuxd to signal that the gateway listener is bound and ready for this User VM.
        #[cfg(not(feature = "standalone"))]
        self.linuxd
            .wait_for_gateway_ready(u32::from(uservm_id), GATEWAY_CONNECT_TIMEOUT)
            .await?;

        Ok(RunningSandbox {
            tag,
            uservm,
            #[cfg(not(feature = "standalone"))]
            _linuxd: self.linuxd,
            #[cfg(not(feature = "standalone"))]
            _control_plane_acceptor: self.control_plane_acceptor,
            gateway_socket_info,
        })
    }

    ///
    /// # Description
    ///
    /// Returns a shared handle to the Linux Daemon instance managing this sandbox.
    ///
    /// # Returns
    ///
    /// A shared handle to the Linux Daemon instance.
    ///
    #[cfg(not(feature = "standalone"))]
    pub fn linuxd(&self) -> Arc<LinuxDaemon> {
        self.linuxd.clone()
    }

    #[cfg(not(feature = "standalone"))]
    ///
    /// # Description
    ///
    /// Returns a shared handle to the control-plane acceptor associated with this sandbox.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Returns
    ///
    /// Returns the shared control-plane acceptor handle.
    ///
    pub fn control_plane_acceptor(&self) -> Arc<ControlPlaneAcceptor> {
        self.control_plane_acceptor.clone()
    }
}
