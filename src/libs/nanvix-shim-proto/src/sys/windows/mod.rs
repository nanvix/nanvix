// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Windows-specific shim protocol implementation.
//!
//! On Windows, containerd communicates with shims via named pipes instead of
//! Unix domain sockets. The address format is `\\.\pipe\containerd-shim-{hash}-pipe`.

use sha2::{
    Digest,
    Sha256,
};

/// Named pipe prefix for containerd shim communication on Windows.
pub const SOCKET_ROOT: &str = r"\\.\pipe\containerd-containerd";

/// Compute a deterministic named pipe address from containerd address, namespace, and id.
pub fn socket_address(address: &str, namespace: &str, id: &str) -> String {
    let data: String = format!("{}\\{}\\{}", address, namespace, id);
    let hash = Sha256::digest(data.as_bytes());
    let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    format!(r"\\.\pipe\containerd-shim-{}-pipe", hex)
}

/// Extract the pipe path from an address string.
pub fn parse_sockaddr(addr: &str) -> &str {
    addr
}

/// Create a named pipe listener at the given address.
///
/// On Windows, the ttrpc server creates the named pipe itself during `Server::start()`.
/// This function just validates the address is not already in use.
///
/// Returns a dummy fd value (-1) since Windows doesn't use raw FDs for named pipes.
pub fn create_listener(address: &str) -> anyhow::Result<i32> {
    use std::{
        fs::OpenOptions,
        os::windows::prelude::OpenOptionsExt,
    };
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OVERLAPPED;

    let mut opts = OpenOptions::new();
    opts.read(true)
        .write(true)
        .custom_flags(FILE_FLAG_OVERLAPPED);

    if opts.open(address).is_ok() {
        anyhow::bail!("named pipe already exists: {}", address);
    }

    // Windows ttrpc creates the pipe; we just return a sentinel
    Ok(-1)
}

/// Signal the parent process that the ttrpc server is ready.
///
/// On Windows, we close stdout by setting it to an invalid handle.
pub fn signal_server_started() {
    // On Windows, closing stdout signals readiness to the parent.
    // The containerd-shim crate uses the same pattern.
    unsafe {
        use windows_sys::Win32::{
            Foundation::INVALID_HANDLE_VALUE,
            System::Console::{
                SetStdHandle,
                STD_OUTPUT_HANDLE,
            },
        };
        SetStdHandle(STD_OUTPUT_HANDLE, INVALID_HANDLE_VALUE);
    }
}
