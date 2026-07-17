// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Socket descriptor handle.

//==================================================================================================
// Structures
//==================================================================================================

/// Routing token for a socket-backed descriptor.
///
/// A socket handle stores the descriptor that `networkd` assigned to the socket (its remote fd),
/// analogous to [`super::HostFsHandle::remote_fd`]. Socket I/O is not served by vfsd; this token
/// only lets vfsd own the descriptor slot and its per-descriptor flags. vfsd closes the remote
/// descriptor on `networkd` when the last reference to the slot is dropped.
pub struct SocketHandle {
    /// Descriptor assigned by `networkd` (the remote fd).
    remote_fd: i32,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SocketHandle {
    /// Creates a socket handle for the given `networkd` descriptor.
    pub fn new(remote_fd: i32) -> Self {
        Self { remote_fd }
    }

    /// Returns the `networkd` descriptor (remote fd) backing this socket.
    pub fn remote_fd(&self) -> i32 {
        self.remote_fd
    }
}
