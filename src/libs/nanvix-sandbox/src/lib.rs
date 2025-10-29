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
//! let config: SandboxConfig = SandboxConfig::new(
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

#[cfg(not(feature = "single-process"))]
pub mod multi_process;
#[cfg(feature = "single-process")]
pub mod single_process;
pub mod tcp_port;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(not(feature = "single-process"))]
pub use self::multi_process::*;
#[cfg(feature = "single-process")]
pub use self::single_process::*;

pub use initialized::InitializedSandbox;
pub use linuxd_args::LinuxDaemonArgs;
pub use running::RunningSandbox;
pub use sandbox_config::SandboxConfig;
pub use uninitialized::UninitializedSandbox;
pub use uservm_args::UserVmArgs;

#[cfg(feature = "single-process")]
pub use ::linuxd::syscalls::SyscallTable;

pub use ::hwloc::HwLoc;
pub use ::syscomm;
