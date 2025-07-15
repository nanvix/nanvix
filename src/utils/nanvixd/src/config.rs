// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

/// Path to the binary directory.
pub const BINARY_DIRECTORY: &str = "./bin";

/// Suffix for Unix sockets.
#[cfg(debug_assertions)]
const UNIX_SOCKET_SUFFIX: &str = ".debug.socket";
#[cfg(not(debug_assertions))]
const UNIX_SOCKET_SUFFIX: &str = ".socket";

/// Path to the temporary directory.
pub const DEFAULT_TMP_DIRECTORY: &str = "/tmp";

pub const HTTP_HEADER_MESSAGE_TYPE: &str = "X-NVX-Message-Type";

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn control_plane_sockaddr_builder(tmp_str: &str, tenant_id: &str) -> String {
    format!("{tmp_str}/control-plane:{tenant_id}:cp{UNIX_SOCKET_SUFFIX}")
}

pub fn user_vm_sockaddr_builder(tmp_str: &str, tenant_id: &str, app_name: &str, sandbox_id: &str) -> String {
    format!("{tmp_str}/{tenant_id}:{app_name}:{sandbox_id}:uvm{UNIX_SOCKET_SUFFIX}")
}

pub fn gateway_sockaddr_builder(tmp_str: &str, tenant_id: &str, app_name: &str, sandbox_id: &str) -> String {
    format!("{tmp_str}/{tenant_id}:{app_name}:{sandbox_id}:gw{UNIX_SOCKET_SUFFIX}")
}
