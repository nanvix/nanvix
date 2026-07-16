// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Terminal interface module for interactive mode.
//!
//! This module provides functionality to run programs in interactive mode, allowing users
//! to directly interact with guest binaries through a terminal interface. It handles
//! terminal raw mode, I/O streaming, and VM lifecycle management.
//!
//! The terminal drives a User VM instance directly via
//! `StandaloneVmHandle`, bypassing the sandbox cache, gateway sockets, and control-plane
//! infrastructure.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

//==================================================================================================
// Modules
//==================================================================================================

mod standalone;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::standalone::Terminal;
