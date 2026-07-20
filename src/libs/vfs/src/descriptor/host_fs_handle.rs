// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Host filesystem descriptor handle.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::String;

//==================================================================================================
// Structures
//==================================================================================================

/// Handle for a file opened on the host filesystem via hostfsd.
///
/// This handle stores the remote file descriptor returned by hostfsd.
/// The VFS cannot perform I/O on this handle directly — all operations
/// must be forwarded via IKC by the owning daemon (vfsd).
///
/// The `is_dir` flag is set once at open time and never re-checked. If the
/// host-side path changes type out-of-band (e.g., replaced by a directory),
/// subsequent operations will use the stale classification.
pub struct HostFsHandle {
    /// Remote file descriptor on the host side.
    remote_fd: i32,
    /// Whether this is a directory.
    is_dir: bool,
    /// Absolute path used to open this handle (stored only for directories to support dirfd).
    path: Option<String>,
    /// Next directory entry index to return on the following `getdents` call.
    readdir_offset: u32,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl HostFsHandle {
    /// Creates a new HostFs handle with the given remote file descriptor.
    ///
    /// The `path` argument is only meaningful for directory handles (used by dirfd resolution).
    /// Pass `None` for regular file handles to avoid unnecessary allocations.
    pub fn new(remote_fd: i32, is_dir: bool, path: Option<String>) -> Self {
        Self {
            remote_fd,
            is_dir,
            path: if is_dir { path } else { None },
            readdir_offset: 0,
        }
    }

    /// Returns the remote file descriptor.
    pub fn remote_fd(&self) -> i32 {
        self.remote_fd
    }

    /// Returns whether this is a directory handle.
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    /// Returns the path used to open this handle (only available for directories).
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Returns the current directory iteration cursor.
    pub fn readdir_offset(&self) -> u32 {
        self.readdir_offset
    }

    /// Sets the directory iteration cursor.
    pub fn set_readdir_offset(&mut self, offset: u32) {
        self.readdir_offset = offset;
    }
}
