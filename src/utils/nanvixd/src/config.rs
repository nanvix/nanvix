// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::error;

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

/// Suffix for Unix sockets.
#[cfg(debug_assertions)]
const UNIX_SOCKET_SUFFIX: &str = ".debug.socket";
#[cfg(not(debug_assertions))]
const UNIX_SOCKET_SUFFIX: &str = ".socket";

/// Path to the temporary directory.
pub const DEFAULT_TMP_DIRECTORY: &str = "/tmp";

pub const HTTP_HEADER_MESSAGE_TYPE: &str = "X-NVX-Message-Type";

/// Maximum length for a Unix socket name, including the null terminator.
/// This is a workaround for the fact that `libc::UNIX_PATH_MAX` is not available.
/// On Linux, this is defined in `<linux/un.h>`.
/// TODO: replace this with `libc::UNIX_PATH_MAX` when it becomes available.
const UNIX_PATH_MAX: usize = 108;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Builds the control plane Unix socket address for a given tenant ID.
///
/// # Arguments
///
/// - tmp_str: Temporary directory path.
/// - tenant_id: Tenant ID.
///
/// # Returns
///
/// On success, returns the name of the control plane Unix socket. On failure, returns an error.
///
pub fn control_plane_sockaddr_builder(tmp_str: &str, tenant_id: &str) -> Result<String> {
    let unix_socket_name: String =
        format!("{tmp_str}/control-plane:{tenant_id}:cp{UNIX_SOCKET_SUFFIX}");

    // Check if socket name exceeds the maximum length.
    if unix_socket_name.len() > UNIX_PATH_MAX {
        let error: String = format!(
            "unix socket name '{unix_socket_name}' exceeds maximum length ({:?} > {:?})",
            unix_socket_name.len(),
            UNIX_PATH_MAX
        );
        error!("control_plane_sockaddr_builder(): {error}");
        anyhow::bail!(error);
    }

    Ok(unix_socket_name)
}

///
/// # Description
///
/// Builds the user VM Unix socket address for a given tenant ID.
///
/// # Arguments
///
/// - tmp_str: Temporary directory path.
/// - tenant_id: Tenant ID.
///
/// # Returns
///
/// On success, returns the name of the user VM Unix socket. On failure, returns an error.
///
pub fn user_vm_sockaddr_builder(tmp_str: &str, tenant_id: &str) -> Result<String> {
    let unix_socket_name: String = format!("{tmp_str}/{tenant_id}:uvm{UNIX_SOCKET_SUFFIX}");

    // Check if socket name exceeds the maximum length.
    if unix_socket_name.len() > UNIX_PATH_MAX {
        let error: String = format!(
            "unix socket name '{unix_socket_name}' exceeds maximum length ({:?} > {:?})",
            unix_socket_name.len(),
            UNIX_PATH_MAX
        );
        error!("user_vm_sockaddr_builder(): {error}");
        anyhow::bail!(error);
    }

    Ok(unix_socket_name)
}

///
/// # Description
///
/// Builds the gateway Unix socket address for a given tenant ID.
///
/// # Arguments
///
/// - tmp_str: Temporary directory path.
/// - tenant_id: Tenant ID.
///
/// # Returns
///
/// On success, returns the name of the gateway Unix socket. On failure, returns an error.
///
pub fn gateway_sockaddr_builder(tmp_str: &str, tenant_id: &str) -> Result<String> {
    let unix_socket_name: String = format!("{tmp_str}/{tenant_id}:gw{UNIX_SOCKET_SUFFIX}");

    // Check if socket name exceeds the maximum length.
    if unix_socket_name.len() > UNIX_PATH_MAX {
        let error: String = format!(
            "unix socket name '{unix_socket_name}' exceeds maximum length ({:?} > {:?})",
            unix_socket_name.len(),
            UNIX_PATH_MAX
        );
        error!("gateway_sockaddr_builder(): {error}");
        anyhow::bail!(error);
    }

    Ok(unix_socket_name)
}
