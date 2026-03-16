// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Unix-specific shim protocol implementation.

use std::{
    os::unix::{
        io::IntoRawFd,
        net::UnixListener,
    },
    path::Path,
};

use sha2::{
    Digest,
    Sha256,
};

/// Root directory for containerd shim sockets on Linux.
pub const SOCKET_ROOT: &str = "/run/containerd";

/// Compute a deterministic socket address from containerd address, namespace, and id.
pub fn socket_address(address: &str, namespace: &str, id: &str) -> String {
    let data: String = format!("{}/{}/{}", address, namespace, id);
    let hash = Sha256::digest(data.as_bytes());
    format!("unix://{}/s/{:x}", SOCKET_ROOT, hash)
}

/// Strip the `unix://` prefix from a socket address to get the filesystem path.
pub fn parse_sockaddr(addr: &str) -> &str {
    if let Some(stripped) = addr.strip_prefix("unix://") {
        return stripped;
    }
    if let Some(stripped) = addr.strip_prefix("vsock://") {
        return stripped;
    }
    addr
}

/// Create a Unix domain socket listener at the given address.
///
/// Returns the raw file descriptor for use with ttrpc `Server::from_raw_fd`.
pub fn create_listener(address: &str) -> anyhow::Result<i32> {
    let socket_path: &str = parse_sockaddr(address);

    if let Some(parent) = Path::new(socket_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove stale socket if it exists
    if Path::new(socket_path).exists() {
        std::fs::remove_file(socket_path)?;
    }

    let listener: UnixListener = UnixListener::bind(socket_path)?;
    Ok(listener.into_raw_fd())
}

/// Signal the parent process that the ttrpc server is ready.
///
/// On Unix, this redirects stdout to stderr via `dup2`. The parent is blocked
/// reading the child's stdout pipe; when stdout is redirected, the pipe closes
/// and the parent gets EOF.
pub fn signal_server_started() {
    unsafe {
        libc::dup2(libc::STDERR_FILENO, libc::STDOUT_FILENO);
    }
}
