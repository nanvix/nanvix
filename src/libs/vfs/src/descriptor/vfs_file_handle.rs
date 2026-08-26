// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Backend-neutral VFS descriptor handle.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    ConsoleHandle,
    DirectReadHandle,
    DirectoryHandle,
    HostFsHandle,
    NullHandle,
    SocketHandle,
};
use crate::{
    filesystem::File,
    pipe::PipeEnd,
    state,
};
use ::fat32::Fat32Error;
use ::sysapi::{
    ffi::c_int,
    sys_types::off_t,
};

//==================================================================================================
// Enumerations
//==================================================================================================

/// An open file handle managed by the VFS.
///
/// Each variant corresponds to a concrete filesystem backend or an
/// optimization path. The VFS FD table stores these handles and
/// dispatches operations to the appropriate variant.
pub enum VfsFileHandle {
    /// File opened through the FAT32 backend.
    Fat32(File),
    /// Zero-copy direct memory read (contiguous file optimization).
    DirectRead(DirectReadHandle),
    /// Open directory handle for `readdir()`/`getdents()` operations.
    Directory(DirectoryHandle),
    /// Remote file opened through the host filesystem daemon (hostfsd).
    /// Operations on this handle must be forwarded via IKC by the caller (vfsd).
    HostFs(HostFsHandle),
    /// The null device.
    Null(NullHandle),
    /// One end of a POSIX unnamed pipe.
    Pipe(PipeEnd),
    /// Routing token for a console stream (stdin/stdout/stderr).
    Console(ConsoleHandle),
    /// Routing token for a socket, holding the descriptor assigned by `networkd`.
    Socket(SocketHandle),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl VfsFileHandle {
    /// Reads data from the file.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => file.read(buf),
            VfsFileHandle::DirectRead(handle) => state::with_storage_lock(|| Ok(handle.read(buf))),
            VfsFileHandle::Directory(_) => Err(Fat32Error::NotSupported),
            VfsFileHandle::HostFs(_) => Err(Fat32Error::NotSupported),
            VfsFileHandle::Null(handle) => handle.read(),
            VfsFileHandle::Pipe(_) => Err(Fat32Error::NotSupported),
            VfsFileHandle::Console(_) | VfsFileHandle::Socket(_) => Err(Fat32Error::NotSupported),
        }
    }

    /// Writes data to the file.
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => file.write(buf),
            VfsFileHandle::DirectRead(_) => Err(Fat32Error::ReadOnly),
            VfsFileHandle::Directory(_) => Err(Fat32Error::NotSupported),
            VfsFileHandle::HostFs(_) => Err(Fat32Error::NotSupported),
            VfsFileHandle::Null(handle) => handle.write(buf),
            VfsFileHandle::Pipe(_) => Err(Fat32Error::NotSupported),
            VfsFileHandle::Console(_) | VfsFileHandle::Socket(_) => Err(Fat32Error::NotSupported),
        }
    }

    /// Seeks to a position in the file.
    pub fn seek(&mut self, offset: off_t, whence: c_int) -> Result<off_t, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => {
                let pos: u64 = file.seek(whence, offset)?;
                Ok(pos as off_t)
            },
            VfsFileHandle::DirectRead(handle) => handle.seek(offset, whence),
            VfsFileHandle::Directory(_) => Err(Fat32Error::NotSupported),
            VfsFileHandle::HostFs(_) => Err(Fat32Error::NotSupported),
            VfsFileHandle::Null(handle) => handle.seek(whence),
            VfsFileHandle::Pipe(_) => Err(Fat32Error::NotSupported),
            VfsFileHandle::Console(_) | VfsFileHandle::Socket(_) => Err(Fat32Error::NotSupported),
        }
    }

    /// Returns the file size in bytes.
    pub fn size(&mut self) -> Result<u64, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => file.size(),
            VfsFileHandle::DirectRead(handle) => Ok(handle.size() as u64),
            VfsFileHandle::Directory(_) => Ok(0),
            VfsFileHandle::HostFs(_) => Ok(0),
            VfsFileHandle::Null(_) => Ok(0),
            VfsFileHandle::Pipe(_) => Ok(0),
            VfsFileHandle::Console(_) | VfsFileHandle::Socket(_) => Ok(0),
        }
    }

    /// Returns whether this handle is a directory.
    pub fn is_dir(&self) -> bool {
        match self {
            VfsFileHandle::Directory(_) => true,
            VfsFileHandle::HostFs(handle) => handle.is_dir(),
            _ => false,
        }
    }

    /// Returns whether this handle is backed by the host filesystem.
    pub fn is_hostfs(&self) -> bool {
        matches!(self, VfsFileHandle::HostFs(_))
    }

    /// Returns the remote FD if this is a HostFs handle.
    pub fn hostfs_remote_fd(&self) -> Option<i32> {
        match self {
            VfsFileHandle::HostFs(handle) => Some(handle.remote_fd()),
            _ => None,
        }
    }

    /// Returns the pipe end if this handle is one end of a pipe.
    pub fn pipe_end(&self) -> Option<&PipeEnd> {
        match self {
            VfsFileHandle::Pipe(end) => Some(end),
            _ => None,
        }
    }
}
