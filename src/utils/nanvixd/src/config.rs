// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Configuration constants and utilities for Nanvix Daemon.
//!
//! This module provides configuration constants, default values, and utility functions for
//! constructing socket addresses and managing paths used throughout the Nanvix Daemon. It
//! handles both Unix domain sockets and TCP sockets.

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
/// Default path to the temporary directory.
///
#[cfg(unix)]
pub const DEFAULT_TMP_DIRECTORY: &str = "/tmp";

///
/// # Description
///
/// Default path to the temporary directory.
///
#[cfg(windows)]
pub const DEFAULT_TMP_DIRECTORY: &str = ".";

///
/// # Description
///
/// Default base filename for guest console output logs.
///
/// The actual log file will be named as `{DEFAULT_CONSOLE_FILENAME}_YYYY_MM_DD_HH_MM.log`
/// where the timestamp is appended to create unique log files for each guest session.
///
pub const DEFAULT_CONSOLE_FILENAME: &str = "guest";
