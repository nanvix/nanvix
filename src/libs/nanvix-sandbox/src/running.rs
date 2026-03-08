// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Running sandbox state management.
//!
//! This module defines the `RunningSandbox` structure representing a fully operational
//! sandboxed execution environment. It provides methods for accessing sandbox resources
//! and performing graceful shutdown operations.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "standalone"))]
use crate::linuxd::LinuxDaemon;
use crate::{
    tcp_port::TcpPort,
    uservm::UserVm,
    SandboxTag,
};
use ::std::{
    process::ExitStatus,
    sync::Arc,
};
use ::syscomm::{
    SocketListener,
    SocketType,
};
use ::tokio::sync::Mutex;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A running sandbox with an active User VM instance.
///
/// This structure represents a fully operational sandboxed execution environment with a
/// running User VM. It maintains handles to all active components and resources needed for
/// the sandbox to operate, including the User VM, Linux Daemon, and socket connections.
///
pub struct RunningSandbox {
    /// Sandbox tag containing tenant, program, and application information.
    pub(crate) tag: SandboxTag,
    /// Handle to the running User VM instance.
    pub(super) uservm: UserVm,
    /// Shared handle to the Linux Daemon instance (kept alive for resource management).
    #[cfg(not(feature = "standalone"))]
    pub(super) _linuxd: Arc<LinuxDaemon>, // Keep resource.
    /// Gateway socket information (address, socket type, optional L2 TCP port).
    pub(super) gateway_socket_info: (String, SocketType, Option<TcpPort>),
    /// Control plane listener socket, address, and socket type (kept alive for resource management).
    pub(super) _control_plane_socket_and_info: Arc<Mutex<(SocketListener, String, SocketType)>>, // Keep resource.
}

impl RunningSandbox {
    ///
    /// # Description
    ///
    /// Returns a reference to the sandbox tag.
    ///
    /// The sandbox tag contains tenant, program, and application information that uniquely
    /// identifies this sandbox instance.
    ///
    /// # Returns
    ///
    /// A reference to the sandbox tag.
    ///
    pub fn tag(&self) -> &SandboxTag {
        &self.tag
    }

    ///
    /// # Description
    ///
    /// Returns the gateway socket information (address and socket type).
    ///
    /// The gateway socket is used by external clients to communicate with the User VM's
    /// stdin/stdout through the Linux Daemon.
    ///
    /// # Returns
    ///
    /// A tuple containing the gateway socket address and socket type.
    ///
    pub fn gateway_socket_info(&self) -> (String, SocketType) {
        (self.gateway_socket_info.0.clone(), self.gateway_socket_info.1)
    }

    ///
    /// # Description
    ///
    /// Performs a graceful shutdown of the running sandbox by terminating the User VM instance.
    ///
    /// This method consumes the sandbox and ensures all resources are properly cleaned up.
    ///
    /// # Returns
    ///
    /// Returns `Some(ExitStatus)` if the User VM task or process finished gracefully, and
    /// `None` otherwise.
    ///
    pub async fn shutdown(mut self) -> Option<ExitStatus> {
        self.uservm.shutdown().await
    }
}
