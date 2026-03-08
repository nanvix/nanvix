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
use crate::config::GATEWAY_CONNECT_TIMEOUT;
#[cfg(not(feature = "standalone"))]
use crate::linuxd::LinuxDaemon;
#[cfg(not(any(feature = "single-process", feature = "standalone")))]
use crate::netns::NetnsHandle;
use crate::{
    tcp_port::TcpPort,
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
    /// Control plane listener socket, address, and socket type.
    pub(super) control_plane_bind_socket_and_info: Arc<Mutex<(SocketListener, String, SocketType)>>,
    /// Complete configuration for the sandbox execution environment.
    pub(super) sandbox_config: SandboxConfig<T>,
    /// Handle to the network namespace (only set in L2-mode).
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    pub(super) netns_handle: Option<NetnsHandle>,
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
    #[cfg_attr(
        any(feature = "single-process", feature = "standalone"),
        allow(unused_mut)
    )]
    #[cfg_attr(feature = "standalone", allow(unused_variables))]
    pub async fn start(mut self, tag: SandboxTag) -> Result<RunningSandbox> {
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

        // Extract gateway socket info (consumes the config to get ownership of TcpPort).
        let gateway_socket_info_with_port: (String, SocketType, Option<TcpPort>) =
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
            let mut locked_control_plane_bind_socket_and_info: MutexGuard<
                '_,
                (SocketListener, String, SocketType),
            > = self.control_plane_bind_socket_and_info.lock().await;
            match UserVm::spawn(
                &uservm_args,
                // Pass a mutable reference to the unique control-plane listener socket to accept
                // one connection from the new user VM.
                &mut locked_control_plane_bind_socket_and_info.0,
                // Pass ownership of the netns RAII handle to the user VM.
                #[cfg(not(feature = "single-process"))]
                self.netns_handle.take(),
            )
            .await
            {
                Ok(uservm) => uservm,
                Err(error) => {
                    error!("start(): failed to spawn uservm (error={error:?})");
                    return Err(error);
                },
            }
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
            _control_plane_socket_and_info: self.control_plane_bind_socket_and_info,
            gateway_socket_info: gateway_socket_info_with_port,
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

    ///
    /// # Description
    ///
    /// Returns a shared handle to the control plane socket information including the listener,
    /// socket address, and socket type.
    ///
    /// # Returns
    ///
    /// A shared handle to the control plane listener socket information.
    ///
    pub fn control_plane_bind_socket_info(
        &self,
    ) -> Arc<Mutex<(SocketListener, String, SocketType)>> {
        self.control_plane_bind_socket_and_info.clone()
    }
}
