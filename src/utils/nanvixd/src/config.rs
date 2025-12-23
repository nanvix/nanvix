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

///
/// # Description
///
/// Default base filename for guest console output logs.
///
/// The actual log file will be named as `{DEFAULT_CONSOLE_FILENAME}_YYYY_MM_DD_HH_MM.log`
/// where the timestamp is appended to create unique log files for each guest session.
///
pub const DEFAULT_CONSOLE_FILENAME: &str = "guest";

///
/// # Description
///
/// Default directory name for storing L2 snapshots.
///
pub const DEFAULT_L2_SNAPSHOT_DIRECTORY: &str = "images";

///
/// # Description
///
/// Default path for the L2 snapshot.
///
/// We cannot define this variable as a pub const &str because it depends on
/// SNAPSHOT_NAME which is another build-time constant.
///
pub fn default_l2_snapshot_path() -> String {
    format!("./images/{}", ::nanvix::config::linuxd::SNAPSHOT_NAME)
}
