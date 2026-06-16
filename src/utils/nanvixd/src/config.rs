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
/// Default path for the cloud-hypervisor binary directory.
///
pub const DEFAULT_CLH_BIN_PATH: &str = "./toolchain/bin";

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

///
/// # Description
///
/// Default directory name for storing L2 snapshots.
///
pub const DEFAULT_L2_SNAPSHOT_DIRECTORY: &str = "images";

///
/// # Description
///
/// Default name for snapshot files.
///
/// # Notes
///
/// - This file must be synced with `generate-l2-snapshot.sh` script.
///
pub const DEFAULT_SNAPSHOT_FILE_NAME: &str = "l2_sysvm_initramfs.img";

///
/// # Description
///
/// Default path for the L2 snapshot.
///
/// We cannot define this variable as a pub const &str because it depends on
/// SNAPSHOT_NAME which is another build-time constant.
///
pub fn default_l2_snapshot_path() -> String {
    format!("./{}/{}", DEFAULT_L2_SNAPSHOT_DIRECTORY, ::nanvix::config::linuxd::SNAPSHOT_NAME)
}
