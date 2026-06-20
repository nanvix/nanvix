// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::{
    build_error,
    fat32_to_error_code,
};
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::syscall::{
    fcntl::message::{
        FileAdvisoryInformationRequest,
        FileAdvisoryInformationResponse,
        FileControlRequest,
        FileControlResponse,
        FileSpaceControlRequest,
        FileSpaceControlResponse,
    },
    sys::stat::message::{
        FileChmodRequest,
        FileChmodResponse,
        UpdateFileAccessTimeRequest,
        UpdateFileAccessTimeResponse,
    },
    unistd::message::{
        CloseRequest,
        CloseResponse,
        FileChdirRequest,
        FileChdirResponse,
        FileChownRequest,
        FileChownResponse,
        FileDataSyncRequest,
        FileDataSyncResponse,
        FileSyncRequest,
        FileSyncResponse,
        FileTruncateRequest,
        FileTruncateResponse,
        RegisterSocketRequest,
        RegisterSocketResponse,
        ResolveFdRequest,
        ResolveFdResponse,
        SeekRequest,
        SeekResponse,
    },
    SystemCallMessage,
};
use ::vfs::fd::VfsRoute;

//==================================================================================================
// Short Request Handlers
//==================================================================================================

pub(crate) fn handle_close(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: CloseRequest = CloseRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    match ::vfs::fd::vfs_close(fd) {
        Ok(()) => CloseResponse::build(source, 0, ProcessIdentifier::VFSD, MessageType::Ipc),
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

/// Handles a descriptor-resolution query: reports the authoritative backend of a flat descriptor.
///
/// libposix sends this on a resolution-cache miss once descriptor numbers no longer encode their
/// backend. The answer is taken from vfsd's slot table via [`vfs_resolve`](::vfs::fd::vfs_resolve):
/// the backend route, the descriptor that backend expects, and the current table generation (the
/// coherence epoch). A descriptor with no slot is reported as a bad file descriptor.
pub(crate) fn handle_resolve_fd(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: ResolveFdRequest = ResolveFdRequest::from_bytes(msg.payload);
    let pid: ProcessIdentifier = req.pid;
    let fd: i32 = req.fd;
    ::vfs::fd::set_current_process(pid);
    ::vfs::fd::vfs_seed_root_console(pid);
    match ::vfs::fd::vfs_resolve(fd) {
        Some((route, backend_fd)) => {
            let wire_route: u32 = match route {
                VfsRoute::Console => ResolveFdResponse::ROUTE_CONSOLE,
                VfsRoute::Vfs => ResolveFdResponse::ROUTE_VFS,
                VfsRoute::Socket => ResolveFdResponse::ROUTE_SOCKET,
            };
            let epoch: u64 = ::vfs::fd::vfs_current_generation();
            ResolveFdResponse::build(
                source,
                wire_route,
                backend_fd,
                epoch,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )
        },
        // No slot for this descriptor in the caller's table: it is unroutable.
        None => build_error(source, ErrorCode::BadFile),
    }
}

/// Allocates a flat descriptor slot for a socket endpoint that `networkd` already created.
///
/// libposix sends this as the second step of socket creation: `networkd` owns the endpoint, and
/// vfsd binds it to the lowest free flat descriptor so the socket joins the flat namespace like any
/// other object. The response carries that flat descriptor and the current table generation (the
/// coherence epoch) so the caller can seed its resolution cache with a coherent entry.
pub(crate) fn handle_register_socket(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: RegisterSocketRequest = RegisterSocketRequest::from_bytes(msg.payload);
    let remote_fd: i32 = req.remote_fd;
    match ::vfs::fd::vfs_register_socket(remote_fd) {
        Ok(fd) => {
            let epoch: u64 = ::vfs::fd::vfs_current_generation();
            RegisterSocketResponse::build(
                source,
                fd,
                epoch,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )
        },
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

pub(crate) fn handle_seek(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: SeekRequest = SeekRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let offset: i64 = req.offset;
    let whence: i32 = req.whence;
    match ::vfs::fd::vfs_lseek(fd, offset, whence) {
        Ok(new_offset) => {
            SeekResponse::build(source, new_offset, ProcessIdentifier::VFSD, MessageType::Ipc)
        },
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

pub(crate) fn handle_fsync(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: FileSyncRequest = FileSyncRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    match ::vfs::fd::vfs_fsync(fd) {
        Ok(()) => FileSyncResponse::build(source, 0, ProcessIdentifier::VFSD, MessageType::Ipc),
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

pub(crate) fn handle_fdatasync(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: FileDataSyncRequest = FileDataSyncRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    // fdatasync maps to fsync in our VFS.
    match ::vfs::fd::vfs_fsync(fd) {
        Ok(()) => FileDataSyncResponse::build(source, 0, ProcessIdentifier::VFSD, MessageType::Ipc),
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

pub(crate) fn handle_ftruncate(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: FileTruncateRequest = FileTruncateRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let length = req.length;
    match ::vfs::fd::vfs_ftruncate(fd, length) {
        Ok(()) => FileTruncateResponse::build(source, 0, ProcessIdentifier::VFSD, MessageType::Ipc),
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

pub(crate) fn handle_fallocate(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: FileSpaceControlRequest = FileSpaceControlRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let offset = req.offset;
    let len = req.len;
    match ::vfs::fd::vfs_fallocate(fd, offset, len) {
        Ok(()) => {
            FileSpaceControlResponse::build(source, 0, ProcessIdentifier::VFSD, MessageType::Ipc)
        },
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

pub(crate) fn handle_fadvise(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let _req: FileAdvisoryInformationRequest =
        FileAdvisoryInformationRequest::from_bytes(msg.payload);
    // fadvise is advisory only — always succeed.
    FileAdvisoryInformationResponse::build(source, 0, ProcessIdentifier::VFSD, MessageType::Ipc)
}

pub(crate) fn handle_fcntl(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    use ::sysapi::fcntl::file_control_request::{
        F_DUPFD,
        F_DUPFD_CLOEXEC,
        F_DUPFD_CLOFORK,
    };

    let req: FileControlRequest = FileControlRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let cmd: i32 = req.cmd;
    let arg: i32 = req.arg;
    // The duplication commands are slot-table allocations rather than flag queries: route them to
    // the shared `dup` primitive, which aliases `fd`'s open file description into the lowest free
    // descriptor at or above `arg`. The close-on-exec / close-on-fork variants set the matching
    // per-descriptor flag on the freshly allocated duplicate (which otherwise starts cleared).
    let result: Result<i32, ::vfs::Fat32Error> = match cmd {
        F_DUPFD => ::vfs::fd::vfs_dup_from(fd, arg),
        F_DUPFD_CLOEXEC => dup_from_with_flag(fd, arg, true, false),
        F_DUPFD_CLOFORK => dup_from_with_flag(fd, arg, false, true),
        _ => ::vfs::fd::vfs_fcntl(fd, cmd, arg),
    };
    match result {
        Ok(ret) => {
            FileControlResponse::build(source, ret, ProcessIdentifier::VFSD, MessageType::Ipc)
        },
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

/// Duplicates `fd` into the lowest free descriptor at or above `min_fd`, then sets the requested
/// per-descriptor flags on the duplicate.
///
/// This backs the `F_DUPFD_CLOEXEC` / `F_DUPFD_CLOFORK` `fcntl` commands, which differ from plain
/// `F_DUPFD` only in that the new descriptor is born with close-on-exec or close-on-fork set rather
/// than cleared.
fn dup_from_with_flag(
    fd: i32,
    min_fd: i32,
    close_on_exec: bool,
    close_on_fork: bool,
) -> Result<i32, ::vfs::Fat32Error> {
    let new_fd: i32 = ::vfs::fd::vfs_dup_from(fd, min_fd)?;
    let mut flags: ::vfs::fd::FdFlags = ::vfs::fd::FdFlags::default();
    flags.set_close_on_exec(close_on_exec);
    flags.set_close_on_fork(close_on_fork);
    ::vfs::fd::vfs_set_fd_flags(new_fd, flags)?;
    Ok(new_fd)
}

pub(crate) fn handle_fchmod(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: FileChmodRequest = FileChmodRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let mode = req.mode;
    // fchmod on an fd: our VFS does not support fchmod by fd directly. Return success as a stub.
    ::syslog::trace!("handle_fchmod(): stubbed (fd={}, mode={})", fd, mode);
    FileChmodResponse::build(source, ProcessIdentifier::VFSD, MessageType::Ipc)
}

pub(crate) fn handle_fchdir(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: FileChdirRequest = FileChdirRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    match ::vfs::fd::vfs_fchdir(fd) {
        Ok(()) => FileChdirResponse::build(source, ProcessIdentifier::VFSD, MessageType::Ipc),
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

pub(crate) fn handle_fchown(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: FileChownRequest = FileChownRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    // fchown is a no-op in our VFS.
    ::syslog::trace!("handle_fchown(): stubbed (fd={})", fd);
    FileChownResponse::build(source, ProcessIdentifier::VFSD, MessageType::Ipc)
}

pub(crate) fn handle_futimens(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: UpdateFileAccessTimeRequest =
        match UpdateFileAccessTimeRequest::from_bytes(msg.payload) {
            Ok(req) => req,
            Err(e) => return build_error(source, e.code),
        };
    let fd: i32 = req.fd;
    // futimens is a no-op in our VFS.
    ::syslog::trace!("handle_futimens(): stubbed (fd={})", fd);
    UpdateFileAccessTimeResponse::build(source, 0, ProcessIdentifier::VFSD, MessageType::Ipc)
}
