// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Configuration constants and utilities for Nanvix Daemon.
//!
//! This module provides configuration constants, default values, and utility functions for
//! constructing socket addresses and managing paths used throughout the Nanvix Daemon. It
//! handles both Unix domain sockets and TCP sockets, and supports L2 deployment modes.

//==================================================================================================
// Imports
//==================================================================================================

use ::tokio::time::Duration;

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
/// HTTP header name for message type identification.
///
pub const HTTP_HEADER_MESSAGE_TYPE: &str = "X-NVX-Message-Type";

///
/// # Description
///
/// Timeout for waiting for graceful shutdown of User VM instances.
///
/// We use control-plane messages to synchronize the graceful shutdown of different components.
/// However, if components are faulty or hang, nanvixd cannot block. Instead, we wait for this
/// timeout and revert to non-graceful shutdowns if the timeout is met.
///
pub const CLEANUP_TIMEOUT: Duration = Duration::from_secs(1);

///
/// # Description
///
/// Timeout for accepting connections on the control plane.
///
pub const CONTROL_PLANE_ACCEPT_TIMEOUT: Duration = Duration::from_secs(60);

///
/// # Description
///
/// Timeout for connecting to gateway.
///
pub const GATEWAY_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

///
/// # Description
///
/// Provides the timeout we should use when waiting for Linuxd to shutdown.
///
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(60);
