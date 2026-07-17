// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Backend-neutral descriptor handle types.

//==================================================================================================
// Modules
//==================================================================================================

mod console_handle;
mod console_stream;
mod direct_read_handle;
mod directory_handle;
mod host_fs_handle;
mod socket_handle;
mod vfs_file_handle;
mod vfs_stat;

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use self::{
    console_handle::ConsoleHandle,
    console_stream::ConsoleStream,
    direct_read_handle::DirectReadHandle,
    directory_handle::DirectoryHandle,
    host_fs_handle::HostFsHandle,
    socket_handle::SocketHandle,
    vfs_file_handle::VfsFileHandle,
    vfs_stat::VfsStat,
};
