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
        SeekRequest,
        SeekResponse,
    },
    SystemCallMessage,
};

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
    let req: FileControlRequest = FileControlRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let cmd: i32 = req.cmd;
    let arg: i32 = req.arg;
    match ::vfs::fd::vfs_fcntl(fd, cmd, arg) {
        Ok(ret) => {
            FileControlResponse::build(source, ret, ProcessIdentifier::VFSD, MessageType::Ipc)
        },
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
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
