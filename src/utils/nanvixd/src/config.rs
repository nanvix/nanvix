// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Configuration constants and utilities for Nanvix Daemon.
//!
//! This module provides configuration constants, default values, and utility functions for
//! constructing socket addresses and managing paths used throughout the Nanvix Daemon. It
//! handles both Unix domain sockets and TCP sockets, and supports L2 deployment modes.

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Default binary directory path for Nanvix binaries.
///
pub const DEFAULT_BIN_DIRECTORY: &str = "./bin";

///
/// # Description
///
/// Default binary directory path for toolchain-related binaries.
///
pub const DEFAULT_TOOLCHAIN_BIN_DIRECTORY: &str = "./toolchain/bin";

///
/// # Description
///
/// Default directory for logs
///
pub const DEFAULT_LOG_DIRECTORY: &str = "./logs";

///
/// # Description
///
/// Default path to the temporary directory.
///
pub const DEFAULT_TMP_DIRECTORY: &str = "/tmp";
