// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

/// Path to the temporary directory.
const TMP_DIRECTORY: &str = "/tmp";

/// Path to the binary directory.
pub const BINARY_DIRECTORY: &str = "./bin";

/// Suffix for Unix sockets.
#[cfg(debug_assertions)]
const UNIX_SOCKET_SUFFIX: &str = ".debug.socket";
#[cfg(not(debug_assertions))]
const UNIX_SOCKET_SUFFIX: &str = ".socket";

/// Default keep-alive timeout.
pub const DEFAULT_KEEP_ALIVE_TIMEOUT: u64 = 60;

/// Backlog for Linux Daemon sockets.
pub const LINUXD_SOCKET_BACKLOG: u32 = 1024;

/// Default Linux Daemon socket address.
pub const DEFAULT_LINUXD_SOCKADDR: &str = "127.0.0.1:1234";

/// Default sandbox socket address.
pub const DEFAULT_SANDBOX_SOCKADDR: &str = "127.0.0.1:7070";

/// Default console file.
pub const DEFAULT_CONSOLE_FILE: &str = "/dev/null";

/// Maximum payload size for requests.
/// NOTE: This is a hard limitation for the current protocol.
pub const MAX_PAYLOAD_SIZE: usize = 32;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn sandbox_sockaddr_builder(sandbox_sockaddr: &str) -> String {
    format!("{}/{}{}", TMP_DIRECTORY, sandbox_sockaddr, UNIX_SOCKET_SUFFIX)
}

pub fn linuxd_sockaddr_builder(linuxd_sockaddr: &str, clientid: usize, requestid: usize) -> String {
    format!(
        "{}/{}:{}:{}{}",
        TMP_DIRECTORY, linuxd_sockaddr, clientid, requestid, UNIX_SOCKET_SUFFIX
    )
}
