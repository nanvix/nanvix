// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    net::Ipv4Addr,
    time::Duration,
};

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

///
/// # Description
///
/// Timeout for joining the reader task when closing a user VM connection.
///
pub const READER_TASK_JOIN_TIMEOUT: Duration = Duration::from_secs(1);

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Builds the TCP address where L2-enabled linuxd deployments block waiting to be snapshotted. The
/// L2 VM is deployed inside a separate network namespace, so they must use the IP of the halve of
/// the VETH pair that is inside the namespace.
///
/// # Parameters
///
/// - `veth_ns_ip`: IP that we can connect to from the host to reach services in the L2 VM. If the
///   value is None, it means we can bind to all addresses.
///
/// # Returns
///
/// The address of the socket.
///
pub fn restore_gate_sockaddr_builder(veth_ns_ip: Option<Ipv4Addr>) -> String {
    if let Some(veth_ns_ip) = veth_ns_ip {
        format!("{}:{DEFAULT_RESTORE_GATE_PORT}", veth_ns_ip)
    } else {
        format!("0.0.0.0:{DEFAULT_RESTORE_GATE_PORT}")
    }
}
