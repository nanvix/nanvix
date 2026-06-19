// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! File descriptor range definitions for Nanvix subsystems.

//==================================================================================================
// File Descriptor Ranges
//==================================================================================================

/// Base file descriptor number for socket handles managed by networkd.
///
/// Socket file descriptors occupy the range `[SOCKET_FD_BASE, ..)`. Under the flat descriptor
/// namespace, vfsd-served descriptors are allocated lowest-free below this base, so this range stays
/// reserved for `networkd` and never collides with a vfsd descriptor (the interim reservation until
/// sockets are unified into the flat table).
pub const SOCKET_FD_BASE: i32 = 2048;

/// Returns `true` if the given file descriptor belongs to the socket fd range.
pub fn is_socket_fd(fd: i32) -> bool {
    fd >= SOCKET_FD_BASE
}
