// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Socket descriptor handle.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::sync::atomic::{
    AtomicU64,
    Ordering,
};

//==================================================================================================
// Socket Identity Allocator
//==================================================================================================

/// Source of process-global, monotonically increasing socket identifiers.
///
/// Identifiers are never recycled for the lifetime of the daemon, unlike the descriptors that
/// `networkd` assigns, which it reuses once a socket is closed. Starts at `1` so that `0` is never
/// reported as an inode number.
static NEXT_SOCKET_ID: AtomicU64 = AtomicU64::new(1);

/// Allocates the next unique socket identifier.
fn alloc_socket_id() -> u64 {
    // `fetch_add` returns the previous value, so on wraparound it can yield `0`. Skip it to keep
    // `0` out of the reported inode numbers.
    loop {
        let id: u64 = NEXT_SOCKET_ID.fetch_add(1, Ordering::Relaxed);
        if id != 0 {
            return id;
        }
    }
}

//==================================================================================================
// Structures
//==================================================================================================

/// Routing token for a socket-backed descriptor.
///
/// A socket handle stores the descriptor that `networkd` assigned to the socket (its remote fd),
/// analogous to [`super::HostFsHandle::remote_fd`]. Socket I/O is not served by vfsd; this token
/// only lets vfsd own the descriptor slot, its per-descriptor flags, and the socket's metadata.
/// vfsd closes the remote descriptor on `networkd` when the last reference to the slot is dropped.
///
/// The handle lives in the open socket description, so its identity and creation time are shared
/// by every descriptor duplicated from it or inherited across `fork`.
pub struct SocketHandle {
    /// Descriptor assigned by `networkd` (the remote fd).
    remote_fd: i32,
    /// Unique identity of the open socket description, reported as its inode number.
    socket_id: u64,
    /// Wall-clock creation time (Unix seconds), reported for all three stat times.
    created: i64,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SocketHandle {
    /// Creates a socket handle for the given `networkd` descriptor.
    ///
    /// The handle receives a fresh socket identity, and the current wall-clock time is recorded as
    /// its creation time.
    pub fn new(remote_fd: i32) -> Self {
        Self {
            remote_fd,
            socket_id: alloc_socket_id(),
            created: crate::time::wall_clock_secs(),
        }
    }

    /// Returns the `networkd` descriptor (remote fd) backing this socket.
    pub fn remote_fd(&self) -> i32 {
        self.remote_fd
    }

    /// Returns the unique identity of this socket.
    pub fn socket_id(&self) -> u64 {
        self.socket_id
    }

    /// Returns the wall-clock creation time (Unix seconds) of this socket handle.
    pub fn created(&self) -> i64 {
        self.created
    }
}
