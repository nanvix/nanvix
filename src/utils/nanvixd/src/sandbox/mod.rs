// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Sandbox management for Nanvix Daemon.
//!
//! This module provides the infrastructure for creating, managing, and destroying sandboxed
//! execution environments. It supports both single-process and multi-process modes, and handles
//! the lifecycle of Linux Daemon and User VM instances.

//==================================================================================================
// Private Modules
//==================================================================================================

mod config;
mod initialized;
mod linuxd_args;
mod running;
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

pub use config::SandboxConfig;
pub use initialized::InitializedSandbox;
pub use linuxd_args::LinuxDaemonArgs;
pub use running::RunningSandbox;
pub use uninitialized::UninitializedSandbox;
pub use uservm_args::UserVmArgs;
