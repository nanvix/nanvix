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
//! use ::nanvix_sandbox::{UninitializedSandbox, SandboxConfig};
//! use ::syscomm::SocketType;
//! use ::user_vm_api::UserVmIdentifier;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create configuration.
//! let config: SandboxConfig<()> = SandboxConfig::new(
//!     UserVmIdentifier::new(0),  // uservm_id
//!     ("127.0.0.1:8080".to_string(), SocketType::Tcp, None),  // gateway_socket_info
//!     ("127.0.0.1:8081".to_string(), SocketType::Tcp),  // system_vm_socket_info
//!     None,  // console_file
//!     None,  // hwloc
//!     "/path/to/kernel.elf",  // kernel_binary_path
//!     #[cfg(not(feature = "single-process"))]
//!     "/path/to/linuxd.elf",  // linuxd_binary_path
//!     #[cfg(not(feature = "single-process"))]
//!     "/path/to/uservm.elf",  // uservm_binary_path
//!     "/path/to/logs",  // log_directory
//!     #[cfg(feature = "single-process")]
//!     None,  // syscall_table
//!     Some(("127.0.0.1:8082".to_string(), SocketType::Tcp)),  // control_plane_socket_info
//!     Some("/path/to/toolchain".to_string()),  // toolchain_binary_directory
//!     Some("/tmp".to_string()),  // tmp_directory
//!     Some(false),  // l2
//!     Some("/path/to/l2/snapshot".to_string()), // l2_snapshot_path
//! );
//!
//! // Create and initialize sandbox.
//! let sandbox = UninitializedSandbox::new("/path/to/guest.elf", None)
//!     .with_config(config)
//!     .initialize()
//!     .await?;
//!
//! // Start execution.
//! let running = sandbox.start().await?;
//!
//! // ... communicate with sandbox via gateway socket ...
//!
//! // Shutdown gracefully.
//! running.shutdown().await;
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
//! - Utilities: [`tcp_port`] for managing TCP port allocations

//==================================================================================================
// Imports
//==================================================================================================

use crate::config::NAMED_RESOURCE_PREFIX;
use ::anyhow::Result;
use ::std::{
    borrow::Cow,
    ffi::OsString,
    path::PathBuf,
};
use ::syslog::{
    error,
    warn,
};
use ::tokio::{
    fs,
    fs::ReadDir,
};

//==================================================================================================
// Private Modules
//==================================================================================================

mod config;
mod initialized;
mod linuxd_args;
mod running;
mod sandbox_config;
mod uninitialized;
mod uservm_args;

//==================================================================================================
// Public Modules
//==================================================================================================

::cfg_if::cfg_if! {
    if #[cfg(feature = "single-process")] {
        pub mod single_process;
    } else {
        pub mod multi_process;
    }
}

pub mod tcp_port;

//==================================================================================================
// Exports
//==================================================================================================

::cfg_if::cfg_if! {
    if #[cfg(feature = "single-process")] {
        pub use self::single_process::*;
        pub use ::linuxd::syscalls::SyscallAction;
        pub use ::linuxd::syscalls::SyscallTable;
    } else {
        pub use self::multi_process::*;
    }
}

pub use self::{
    initialized::InitializedSandbox,
    linuxd_args::LinuxDaemonArgs,
    running::RunningSandbox,
    sandbox_config::SandboxConfig,
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
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Helper function that removes dangling resources from previous runs.
///
/// # Parameters
///
/// - `tmp_dir_path`: Path to the temporary directory.
///
/// # Returns
///
/// On success, this function returns an empty tuple. On failure, it returns an object that
/// describes the error.
///
pub async fn remove_dangling_resources(tmp_dir_path: &str) -> Result<()> {
    let mut result: Result<()> = Ok(());

    // Open temporary directory, all named resources are created here.
    let mut dir_entries: ReadDir = fs::read_dir(tmp_dir_path).await.map_err(|error| {
        let reason: String =
            format!("failed to read temporary directory '{tmp_dir_path}': {error}");
        error!("remove_dangling_resources(): {reason}");
        anyhow::anyhow!(reason)
    })?;

    // Traverse directory entries and remove all dangling resources that start with the magic prefix
    // for named resources. If we fail, log an error message and continue, as we want to cleanup as
    // many resources as possible. The overall result will be an error if any removal fails.
    loop {
        match dir_entries.next_entry().await {
            Ok(None) => break,
            Ok(Some(entry)) => {
                let file_name: OsString = entry.file_name();
                let file_name_str: Cow<'_, str> = file_name.to_string_lossy();

                // Check if the file name matches the named resource prefix.
                if file_name_str.starts_with(NAMED_RESOURCE_PREFIX) {
                    let file_path: PathBuf = entry.path();

                    // Attempt to remove dangling resource.
                    if let Err(error) = fs::remove_file(file_path).await {
                        let reason: String = format!(
                            "failed to remove dangling resource '{}': {error}",
                            file_name_str
                        );
                        error!("remove_dangling_resources(): {reason}");

                        // Do not overwrite previous errors.
                        if result.is_ok() {
                            result = Err(anyhow::anyhow!(reason));
                        }
                    } else {
                        warn!("removed dangling resource: {}", file_name_str);
                    }
                }
            },
            Err(error) => {
                let reason: String = format!(
                    "failed to read entry in temporary directory '{}': {error}",
                    tmp_dir_path
                );
                error!("remove_dangling_resources(): {reason}");

                // Do not overwrite previous errors.
                if result.is_ok() {
                    result = Err(anyhow::anyhow!(reason));
                }
            },
        };
    }

    result
}
