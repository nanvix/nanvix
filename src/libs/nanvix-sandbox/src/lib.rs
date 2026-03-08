// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Nanvix Sandbox Management Library
//!
//! This library provides a high-level API for creating and managing sandboxed execution
//! environments in Nanvix. It implements a state machine pattern for sandbox lifecycle
//! management and supports both single-process and multi-process deployment modes.
//!
//! ## Overview
//!
//! A sandbox in Nanvix consists of:
//! - **Linux Daemon (linuxd)**: System service that manages system calls and I/O operations
//! - **User VM (uservm)**: Virtual machine that executes guest programs in isolation
//! - **Control Plane Socket**: Communication channel for management operations
//! - **Gateway Socket**: Communication channel for program I/O
//! - **System VM Socket**: Communication channel between linuxd and uservm
//!
//! ## Sandbox Lifecycle
//!
//! The library implements a type-safe state machine with three states:
//!
//! 1. **Uninitialized** ([`UninitializedSandbox`]): Initial configuration stage
//!    - Configure sandbox parameters ([`SandboxConfig`])
//!    - Optionally attach existing Linux Daemon or control plane socket
//!    - Builder pattern for flexible configuration
//!
//! 2. **Initialized** ([`InitializedSandbox`]): Ready to start execution
//!    - Control plane socket is bound and listening
//!    - Linux Daemon is spawned (or reused from existing instance)
//!    - Ready to spawn User VM
//!
//! 3. **Running** ([`RunningSandbox`]): Actively executing guest program
//!    - User VM is running the guest program
//!    - All communication channels are active
//!    - Can be shut down gracefully
//!
//! ## Deployment Modes
//!
//! ### Multi-Process Mode (default)
//!
//! Linux Daemon and User VM run as separate OS processes. This is the production mode
//! used by Nanvix Daemon for isolation and robustness.
//!
//! ### Single-Process Mode
//!
//! Linux Daemon and User VM run as tasks within the same process. Enabled via the
//! `single-process` feature flag. This mode is primarily used for testing and development.
//!
//! ## Basic Usage Example
//!
//! ```rust,no_run
//! use ::nanvix_sandbox::{UninitializedSandbox, SandboxConfig, SandboxTag};
//! use ::syscomm::{SocketListener, SocketType, UnboundSocket};
//! use ::user_vm_api::UserVmIdentifier;
//! use ::std::sync::Arc;
//! use ::tokio::sync::Mutex;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create configuration.
//! let config: SandboxConfig<()> = SandboxConfig::new(
//!     "tenant-1",  // tenant_id
//!     UserVmIdentifier::new(0),  // uservm_id
//!     ("127.0.0.1:8080".to_string(), SocketType::Tcp, None),  // gateway_socket_info
//!     ("127.0.0.1:8081".to_string(), SocketType::Tcp),  // system_vm_socket_info
//!     None,  // console_file
//!     None,  // hwloc
//!     "/path/to/kernel.elf",  // kernel_binary_path
//!     #[cfg(not(any(feature = "single-process", feature = "standalone")))]
//!     "/path/to/linuxd.elf",  // linuxd_binary_path
//!     #[cfg(not(any(feature = "single-process", feature = "standalone")))]
//!     "/path/to/uservm.elf",  // uservm_binary_path
//!     "/path/to/logs",  // log_directory
//!     #[cfg(feature = "single-process")]
//!     None,  // syscall_table
//!     Some(("127.0.0.1:8082".to_string(), SocketType::Tcp)),  // control_plane_bind_socket_info
//!     ("127.0.0.1:8081".to_string(), SocketType::Tcp),  // control_plane_connect_socket_info
//!     Some("/path/to/toolchain".to_string()),  // toolchain_binary_directory
//!     Some("/tmp".to_string()),  // tmp_directory
//!     Some(false),  // l2
//! );
//!
//! // Create and bind control plane socket.
//! let control_plane_bind_sockaddr: String = "/tmp/nvx:cp.socket".to_string();
//! let control_plane_socket: SocketListener =
//!     UnboundSocket::new(SocketType::Unix).bind(&control_plane_bind_sockaddr).await?;
//! let control_plane_bind_socket: Arc<Mutex<(SocketListener, String, SocketType)>> =
//!     Arc::new(Mutex::new((control_plane_socket, control_plane_bind_sockaddr, SocketType::Unix)));
//!
//! // Create and initialize sandbox.
//! let sandbox = UninitializedSandbox::new(
//!     "/path/to/guest.elf",
//!     None,
//!     None,
//!     control_plane_bind_socket,
//! )
//!     .with_config(config)
//!     .initialize()
//!     .await?;
//!
//! // Start execution.
//! let tag: SandboxTag = SandboxTag::new("tenant-1", "/path/to/guest.elf", "my-app", None);
//! let running = sandbox.start(tag).await?;
//!
//! // ... communicate with sandbox via gateway socket ...
//!
//! // Shutdown gracefully.
//! let exit_status = running.shutdown().await;
//! // Inspect `exit_status` as needed for diagnostics.
//! # Ok(())
//! # }
//! ```
//!
//! ## Features
//!
//! - **`single-process`**: Enable single-process deployment mode
//! - **`hyperlight`**: Enable Hyperlight virtualization backend support
//!
//! ## Architecture
//!
//! The library is organized into several modules:
//!
//! - Core state types: [`UninitializedSandbox`], [`InitializedSandbox`], [`RunningSandbox`]
//! - Configuration: [`SandboxConfig`], [`LinuxDaemonArgs`], [`UserVmArgs`]
//! - Implementation: [`multi_process`] (default), [`single_process`] (feature-gated)
//! - Utilities: [`netns`] for managing network namespaces, [`tcp_port`] for managing TCP port allocations

//==================================================================================================
// Private Modules
//==================================================================================================

mod config;
mod initialized;
#[cfg(not(feature = "standalone"))]
mod linuxd_args;
mod running;
mod sandbox_config;
#[cfg(not(any(feature = "single-process", feature = "standalone")))]
mod snapshot_dir_handle;
mod tag;
mod uninitialized;
mod uservm_args;

//==================================================================================================
// Public Modules
//==================================================================================================

::cfg_if::cfg_if! {
    if #[cfg(feature = "standalone")] {
        pub mod standalone;
    } else {
        pub mod sandbox;
    }
}

pub mod netns;
pub mod tcp_port;

::cfg_if::cfg_if! {
    if #[cfg(feature = "single-process")] {
        pub mod simple_cache;
    }
}

//==================================================================================================
// Exports
//==================================================================================================

::cfg_if::cfg_if! {
    if #[cfg(feature = "standalone")] {
        pub use self::standalone::*;
    } else if #[cfg(feature = "single-process")] {
        pub use self::sandbox::single_process::*;
        pub use ::linuxd::syscalls::SyscallAction;
        pub use ::linuxd::syscalls::SyscallTable;
    } else {
        pub use self::sandbox::multi_process::*;
    }
}

#[cfg(not(any(feature = "single-process", feature = "standalone")))]
pub use self::snapshot_dir_handle::SnapshotDirHandle;

#[cfg(not(feature = "standalone"))]
pub use self::linuxd_args::LinuxDaemonArgs;
pub use self::{
    initialized::InitializedSandbox,
    running::RunningSandbox,
    sandbox_config::SandboxConfig,
    tag::SandboxTag,
    uninitialized::UninitializedSandbox,
    uservm_args::UserVmArgs,
};
pub use ::hwloc::HwLoc;
pub use ::syscomm;
pub use ::user_vm_api::UserVmIdentifier;
pub use config::{
    control_plane_sockaddr_builder,
    gateway_sockaddr_builder,
    user_vm_sockaddr_builder,
    NAMED_RESOURCE_PREFIX,
    NETNS_NAME_PREFIX,
    UNIX_SOCKET_SUFFIX,
    VETH_HOST_PREFIX,
    VETH_NS_PREFIX,
};
