// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Port where linuxd, when deployed inside an L2 VM, will block waiting for a snapshot to happen.
///
const DEFAULT_RESTORE_GATE_PORT: u32 = 5555;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the IP address where services inside the L2 system VM may bind to.
///
/// # Returns
///
/// The system VM's guest bind IP.
///
pub fn l2_system_vm_guest_ip() -> String {
    config::linuxd::GUEST_TAP_IP_ADDRESS.to_string()
}

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
    format!("{}:{DEFAULT_RESTORE_GATE_PORT}", l2_system_vm_guest_ip())
}
