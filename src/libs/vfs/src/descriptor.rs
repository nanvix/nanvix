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
mod fd_flags;
mod host_fs_handle;
mod pipe_closure;
mod process_exit_reclaim;
mod socket_handle;
mod tty_error;
mod vfs_file_handle;
mod vfs_route;
mod vfs_stat;

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use self::{
    console_handle::ConsoleHandle,
    console_stream::ConsoleStream,
    direct_read_handle::DirectReadHandle,
    directory_handle::DirectoryHandle,
    fd_flags::FdFlags,
    host_fs_handle::HostFsHandle,
    pipe_closure::PipeClosure,
    process_exit_reclaim::ProcessExitReclaim,
    socket_handle::SocketHandle,
    tty_error::TtyError,
    vfs_file_handle::VfsFileHandle,
    vfs_route::VfsRoute,
    vfs_stat::VfsStat,
};
