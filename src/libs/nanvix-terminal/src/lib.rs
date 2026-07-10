// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Terminal interface module for interactive mode.
//!
//! This module provides functionality to run programs in interactive mode, allowing users
//! to directly interact with guest binaries through a terminal interface. It handles
//! terminal raw mode, I/O streaming, and VM lifecycle management.
//!
//! In standalone mode, the terminal drives a User VM instance directly via
//! `StandaloneVmHandle`, bypassing the sandbox cache, gateway sockets, and control-plane
//! infrastructure. In single-process mode, the terminal connects to a
//! gateway socket provided by the sandbox cache and streams I/O between stdin/stdout and
//! the gateway.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

#[cfg(all(feature = "standalone", feature = "single-process"))]
compile_error!("features `standalone` and `single-process` are mutually exclusive");

#[cfg(not(any(feature = "standalone", feature = "single-process")))]
compile_error!("one of the features `standalone` or `single-process` must be enabled");

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(feature = "single-process")]
mod single_process;
#[cfg(feature = "standalone")]
mod standalone;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "single-process")]
pub use self::single_process::Terminal;
#[cfg(feature = "standalone")]
pub use self::standalone::Terminal;
