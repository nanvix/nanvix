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

use crate::{
    config::GATEWAY_CONNECT_TIMEOUT,
    linuxd::LinuxDaemon,
    tcp_port::TcpPort,
    uservm::UserVm,
    RunningSandbox,
    SandboxConfig,
    UserVmArgs,
};
use ::anyhow::Result;
use ::std::sync::Arc;
use ::syscomm::{
    SocketListener,
    SocketType,
    UnboundSocket,
};
use ::syslog::{
    debug,
    error,
};
use ::tokio::{
    sync::{
        Mutex,
        MutexGuard,
    },
    time::Instant,
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
pub struct InitializedSandbox {
    /// Path to the guest binary file to execute.
    pub(super) guest_binary_path: String,
    /// Path to the kernel binary file.
    pub(super) kernel_binary_path: String,
    /// Optional command-line arguments for the program.
    pub(super) program_args: Option<String>,
    /// Shared handle to the Linux Daemon instance managing this sandbox.
    pub(super) linuxd: Arc<LinuxDaemon>,
    /// Control plane listener socket, address, and socket type.
    pub(super) control_plane_socket_and_info: Arc<Mutex<(SocketListener, String, SocketType)>>,
    /// Complete configuration for the sandbox execution environment.
    pub(super) sandbox_config: SandboxConfig,
}

impl InitializedSandbox {
    ///
    /// # Description
    ///
    /// Starts the sandbox by spawning a User VM instance and waiting for the gateway socket
    /// to become available. This transitions the sandbox from initialized to running state.
    ///
    /// # Returns
    ///
    /// On success, returns a running sandbox with an active User VM. On failure, returns an
    /// error describing what went wrong during startup.
    ///
    pub async fn start(self) -> Result<RunningSandbox> {
        // Extract gateway socket info parts for later use.
        let gateway_sockaddr: String = self.sandbox_config.gateway_socket_info().0.clone();
        let gateway_socket_type: SocketType = self.sandbox_config.gateway_socket_info().1;
        let system_vm_socket_info: (String, SocketType) =
            self.sandbox_config.system_vm_socket_info().clone();
        let console_file: Option<String> =
            self.sandbox_config.console_file().map(|s| s.to_string());
        let hwloc: Option<hwloc::HwLoc> = self.sandbox_config.hwloc();
        let log_directory: String = self.sandbox_config.log_directory().to_string();
        let uservm_id: ::user_vm_api::UserVmIdentifier = self.sandbox_config.uservm_id();
        #[cfg(not(feature = "single-process"))]
        let uservm_binary_path: String = self.sandbox_config.uservm_binary_path().to_string();

        // Extract gateway socket info (consumes the config to get ownership of TcpPort).
        let gateway_socket_info_with_port: (String, SocketType, Option<TcpPort>) =
            self.sandbox_config.into_gateway_socket_info();

        // Spawn User VM.
        let mut uservm: UserVm = {
            let mut locked_control_plane_socket_and_info: MutexGuard<
                '_,
                (SocketListener, String, SocketType),
            > = self.control_plane_socket_and_info.lock().await;
            match UserVm::spawn(
                &UserVmArgs::new(
                    (
                        locked_control_plane_socket_and_info.1.clone(),
                        locked_control_plane_socket_and_info.2,
                    ),
                    (gateway_sockaddr.clone(), gateway_socket_type),
                    system_vm_socket_info,
                    self.guest_binary_path.clone(),
                    self.program_args.clone(),
                    console_file,
                    hwloc,
                    self.kernel_binary_path.clone(),
                    #[cfg(not(feature = "single-process"))]
                    uservm_binary_path,
                    log_directory,
                    uservm_id,
                ),
                &mut locked_control_plane_socket_and_info.0,
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

        // Attempt to connect to the gateway socket.
        wait_for_gateway_connection(
            &mut uservm,
            UnboundSocket::new(gateway_socket_type),
            &gateway_sockaddr,
        )
        .await?;

        Ok(RunningSandbox {
            uservm,
            _linuxd: self.linuxd,
            _control_plane_socket_and_info: self.control_plane_socket_and_info,
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
    /// A shared handle to the control plane socket information.
    ///
    pub fn control_plane_socket_info(&self) -> Arc<Mutex<(SocketListener, String, SocketType)>> {
        self.control_plane_socket_and_info.clone()
    }
}

///
/// # Description
///
/// Waits for the gateway socket to become available by repeatedly attempting to connect to it.
///
/// This function implements a timeout-based retry mechanism to ensure the gateway socket is
/// listening and ready to accept connections before returning success.
///
/// # Parameters
///
/// - `uservm`: Reference to the User VM instance.
/// - `unbound_gateway_socket`: The unbound socket to use for connection attempts.
/// - `gateway_sockaddr`: The address of the gateway socket to connect to.
///
/// # Returns
///
/// On success, returns an empty tuple indicating the gateway is available. On failure or
/// timeout, returns an error describing the connection failure.
///
async fn wait_for_gateway_connection(
    uservm: &mut UserVm,
    unbound_gateway_socket: UnboundSocket,
    gateway_sockaddr: &str,
) -> Result<()> {
    let now: Instant = Instant::now();
    loop {
        // Check if user VM finished before attempting to connect to gateway.
        if !uservm.is_running() {
            let reason: String = format!(
                "user VM terminated before gateway socket became available \
                 (address={gateway_sockaddr})"
            );
            error!("wait_for_gateway_connection(): {reason}");
            return Err(anyhow::anyhow!("{reason}"));
        }

        match unbound_gateway_socket
            .clone()
            .connect(gateway_sockaddr)
            .await
        {
            Ok(_stream) => {
                // Connection successful.
                break;
            },
            Err(_e) => {
                // Connection failed. Sleep a bit and retry.
                if now.elapsed().as_secs() > GATEWAY_CONNECT_TIMEOUT.as_secs() {
                    let reason: String =
                        format!("failed to connect to gateway socket (address={gateway_sockaddr})");
                    error!("wait_for_gateway_connection(): {reason}");
                    return Err(anyhow::anyhow!("{reason}"));
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            },
        }
    }

    debug!("gateway socket file appeared after {:?} (path={:?})", now.elapsed(), &gateway_sockaddr);

    Ok(())
}
