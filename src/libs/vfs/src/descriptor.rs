// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Backend-neutral descriptor handle types.

//==================================================================================================
// Modules
//==================================================================================================

mod access_mode;
mod console_handle;
mod console_stream;
mod direct_read_handle;
mod directory_handle;
mod fd_flags;
mod host_fs_handle;
mod null_handle;
mod pipe_closure;
mod process_exit_reclaim;
mod socket_handle;
mod terminal_handle;
mod tty_error;
mod vfs_file_handle;
mod vfs_route;

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use self::{
    access_mode::AccessMode,
    console_handle::ConsoleHandle,
    console_stream::ConsoleStream,
    direct_read_handle::DirectReadHandle,
    directory_handle::DirectoryHandle,
    fd_flags::FdFlags,
    host_fs_handle::HostFsHandle,
    null_handle::NullHandle,
    pipe_closure::PipeClosure,
    process_exit_reclaim::ProcessExitReclaim,
    socket_handle::SocketHandle,
    terminal_handle::{
        TerminalDevice,
        TerminalHandle,
    },
    tty_error::TtyError,
    vfs_file_handle::VfsFileHandle,
    vfs_route::VfsRoute,
};
