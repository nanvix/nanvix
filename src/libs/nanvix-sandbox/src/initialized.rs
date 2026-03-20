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
use crate::{
    config::GATEWAY_CONNECT_TIMEOUT,
    linuxd::LinuxDaemon,
};
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
use ::log::{
    debug,
    error,
    trace,
};
#[cfg(not(any(feature = "single-process", feature = "standalone")))]
use ::std::marker::PhantomData;
use ::std::sync::Arc;
use ::syscomm::{
    SocketListener,
    SocketType,
    UnboundSocket,
};
use ::user_vm_api::RingTransportKind;
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
        #[cfg(feature = "ring-buffer")]
        let ring_transport_kind: RingTransportKind = if matches!(self.sandbox_config.l2(), Some(true))
        {
            if std::env::var_os("NANVIX_L2_IVSHMEM_PATH").is_some()
                && std::env::var_os("NANVIX_L2_IVSHMEM_SIZE").is_some()
            {
                RingTransportKind::Ivshmem
            } else {
                RingTransportKind::Disabled
            }
        } else {
            RingTransportKind::FilePath
        };
        #[cfg(feature = "ring-buffer")]
        let disable_ring_buffer: bool = ring_transport_kind == RingTransportKind::Disabled;
        #[cfg(not(feature = "ring-buffer"))]
        let ring_transport_kind: RingTransportKind = RingTransportKind::Disabled;
        #[cfg(not(feature = "ring-buffer"))]
        let disable_ring_buffer: bool = false;

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
            ring_transport_kind,
            disable_ring_buffer,
        );

        // Spawn User VM.
        //
        // In standalone mode, the VM runs without any external connections so there is no
        // need to acquire the control-plane listener or wait for the gateway.
        #[cfg(feature = "standalone")]
        let mut uservm: UserVm = match UserVm::spawn(&uservm_args).await {
            Ok(uservm) => uservm,
            Err(error) => {
                error!("start(): failed to spawn uservm (error={error:?})");
                return Err(error);
            },
        };

        #[cfg(not(feature = "standalone"))]
        let mut uservm: UserVm = {
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

        // Attempt to connect to the gateway socket.
        #[cfg(not(feature = "standalone"))]
        wait_for_gateway_connection(
            &mut uservm,
            UnboundSocket::new(gateway_socket_type),
            &gateway_sockaddr,
        )
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
#[cfg(not(feature = "standalone"))]
async fn wait_for_gateway_connection(
    uservm: &mut UserVm,
    unbound_gateway_socket: UnboundSocket,
    gateway_sockaddr: &str,
) -> Result<()> {
    trace!(
        "wait_for_gateway_connection(): waiting for gateway socket to become available \
         (address={gateway_sockaddr})"
    );
    let now: Instant = Instant::now();
    loop {
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
            Ok(_stream) => break,
            Err(_e) => {
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

    debug!(
        "gateway socket file appeared after {:?} (path={:?})",
        now.elapsed(),
        &gateway_sockaddr
    );

    Ok(())
}
