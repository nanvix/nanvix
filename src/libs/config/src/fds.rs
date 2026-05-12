// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! File descriptor range definitions for Nanvix subsystems.

//==================================================================================================
// File Descriptor Ranges
//==================================================================================================

/// Base file descriptor number for VFS-managed handles.
///
/// VFS file descriptors occupy the range `[VFS_FD_BASE, VFS_FD_BASE + VFS_MAX_OPEN_FILES)`.
/// This range must not overlap with stdio (0–2), socket, or linuxd-assigned file descriptors.
pub const VFS_FD_BASE: i32 = 1024;

/// Maximum number of simultaneously open VFS files.
pub const VFS_MAX_OPEN_FILES: usize = 64;

/// Base file descriptor number for socket handles managed by networkd.
///
/// Socket file descriptors occupy the range `[SOCKET_FD_BASE, ..)`.
/// This range must not overlap with stdio (0–2) or VFS-managed descriptors.
pub const SOCKET_FD_BASE: i32 = 2048;

/// Returns `true` if the given file descriptor belongs to the VFS fd range.
pub fn is_vfs_fd(fd: i32) -> bool {
    fd >= VFS_FD_BASE && fd < VFS_FD_BASE + VFS_MAX_OPEN_FILES as i32
}

/// Returns `true` if the given file descriptor belongs to the socket fd range.
pub fn is_socket_fd(fd: i32) -> bool {
    fd >= SOCKET_FD_BASE
}
