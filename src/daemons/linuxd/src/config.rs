// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::time::Duration;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Port where linuxd, when deployed inside an L2 VM, will block waiting for a snapshot to happen.
///
const DEFAULT_RESTORE_GATE_PORT: u32 = 5555;

///
/// # Description
///
/// Default directory for logs
///
pub(crate) const DEFAULT_LOG_DIRECTORY: &str = "./logs";

///
/// # Description
///
/// Timeout for connecting to control-plane.
///
pub const CONTROL_PLANE_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Builds the TCP address where L2-enabled linuxd deployments block waiting to be snapshotted.
///
/// # Returns
///
/// On success, returns the address of the socket. On failure, returns an error.
///
pub fn restore_gate_sockaddr_builder() -> String {
    format!("{}:{DEFAULT_RESTORE_GATE_PORT}", config::linuxd::GUEST_TAP_IP_ADDRESS.to_string())
}
