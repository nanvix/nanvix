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
use ::sysapi::sys_stat::stat;
use ::syscall::{
    dirent::message::{
        GetDirectoryEntriesRequest,
        GetDirectoryEntriesResponse,
    },
    fcntl::message::{
        OpenAtRequest,
        OpenAtResponse,
        RenameAtRequest,
        RenameAtResponse,
        UnlinkAtRequest,
        UnlinkAtResponse,
    },
    message::MessagePartitioner,
    sys::stat::message::{
        FileChmodAtRequest,
        FileChmodAtResponse,
        FileStatAtRequest,
        FileStatAtResponse,
        FileStatRequest,
        MakeDirectoryAtRequest,
        MakeDirectoryAtResponse,
        UpdateFileAccessTimeAtRequest,
        UpdateFileAccessTimeAtResponse,
    },
    unistd::message::{
        ChangeDirectoryRequest,
        ChangeDirectoryResponse,
        FileAccessAtRequest,
        FileAccessAtResponse,
        FileChownAtRequest,
        FileChownAtResponse,
        GetCurrentWorkingDirectoryResponse,
        LinkAtRequest,
        LinkAtResponse,
        ReadLinkAtRequest,
        SymbolicLinkAtRequest,
        SymbolicLinkAtResponse,
    },
    SystemCallMessage,
};
use alloc::{
    vec,
    vec::Vec,
};

//==================================================================================================
// Long Response Handlers (single request, multi-part response)
//==================================================================================================

pub(crate) fn handle_fstat(source: ThreadIdentifier, msg: SystemCallMessage) -> Vec<Message> {
    let req: FileStatRequest = FileStatRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;

    let mut st = stat::default();
    match ::vfs::fd::vfs_fstat(fd, &mut st) {
        Ok(()) => {
            let response: FileStatAtResponse = FileStatAtResponse::new(st);
            match response.into_parts(source, ProcessIdentifier::VFSD, MessageType::Ipc) {
                Ok(parts) => parts,
                Err(e) => {
                    ::syslog::error!("handle_fstat(): into_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::IoErr)]
                },
            }
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

pub(crate) fn handle_getcwd(source: ThreadIdentifier) -> Vec<Message> {
    match ::vfs::fd::vfs_getcwd() {
        Ok(cwd) => match GetCurrentWorkingDirectoryResponse::new(&cwd) {
            Ok(response) => {
                match response.into_parts(source, ProcessIdentifier::VFSD, MessageType::Ipc) {
                    Ok(parts) => parts,
                    Err(e) => {
                        ::syslog::error!("handle_getcwd(): into_parts failed (error={:?})", e);
                        vec![build_error(source, ErrorCode::IoErr)]
                    },
                }
            },
            Err(e) => {
                ::syslog::error!("handle_getcwd(): response creation failed (error={:?})", e);
                vec![build_error(source, ErrorCode::IoErr)]
            },
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

pub(crate) fn handle_getdents(source: ThreadIdentifier, msg: SystemCallMessage) -> Vec<Message> {
    let req: GetDirectoryEntriesRequest = GetDirectoryEntriesRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let count: usize = req.count as usize;

    match ::vfs::fd::vfs_getdents(fd, count) {
        Ok(entries) => {
            let response: GetDirectoryEntriesResponse = GetDirectoryEntriesResponse::new(entries);
            match response.into_parts(source, ProcessIdentifier::VFSD, MessageType::Ipc) {
                Ok(parts) => parts,
                Err(e) => {
                    ::syslog::error!("handle_getdents(): into_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::IoErr)]
                },
            }
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

//==================================================================================================
// Long Request Handlers (multi-part request, single or multi-part response)
//==================================================================================================

pub(crate) fn handle_openat(source: ThreadIdentifier, request: OpenAtRequest) -> Vec<Message> {
    match ::vfs::fd::vfs_openat(request.dirfd, &request.pathname, request.flags) {
        Ok(fd) => {
            vec![OpenAtResponse::build(
                source,
                fd,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )]
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

pub(crate) fn handle_renameat(source: ThreadIdentifier, request: RenameAtRequest) -> Vec<Message> {
    match ::vfs::fd::vfs_renameat(
        request.olddirfd,
        &request.oldpath,
        request.newdirfd,
        &request.newpath,
    ) {
        Ok(()) => {
            vec![RenameAtResponse::build(
                source,
                0,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )]
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

pub(crate) fn handle_unlinkat(source: ThreadIdentifier, request: UnlinkAtRequest) -> Vec<Message> {
    match ::vfs::fd::vfs_unlinkat(request.dirfd, &request.pathname, request.flags) {
        Ok(()) => {
            vec![UnlinkAtResponse::build(
                source,
                0,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )]
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

pub(crate) fn handle_mkdirat(
    source: ThreadIdentifier,
    request: MakeDirectoryAtRequest,
) -> Vec<Message> {
    match ::vfs::fd::vfs_mkdirat(request.dirfd, &request.pathname) {
        Ok(()) => {
            vec![MakeDirectoryAtResponse::build(
                source,
                0,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )]
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

pub(crate) fn handle_fstatat(source: ThreadIdentifier, request: FileStatAtRequest) -> Vec<Message> {
    let mut st = stat::default();
    match ::vfs::fd::vfs_fstatat(request.dirfd, &request.path, &mut st) {
        Ok(()) => {
            let response: FileStatAtResponse = FileStatAtResponse::new(st);
            match response.into_parts(source, ProcessIdentifier::VFSD, MessageType::Ipc) {
                Ok(parts) => parts,
                Err(e) => {
                    ::syslog::error!("handle_fstatat(): into_parts failed (error={:?})", e);
                    vec![build_error(source, ErrorCode::IoErr)]
                },
            }
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

pub(crate) fn handle_chdir(
    source: ThreadIdentifier,
    request: ChangeDirectoryRequest,
) -> Vec<Message> {
    match ::vfs::fd::vfs_chdir(&request.path) {
        Ok(()) => {
            vec![ChangeDirectoryResponse::build(
                source,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )]
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

pub(crate) fn handle_faccessat(
    source: ThreadIdentifier,
    request: FileAccessAtRequest,
) -> Vec<Message> {
    match ::vfs::fd::vfs_accessat(request.dirfd, &request.path) {
        Ok(()) => {
            vec![FileAccessAtResponse::build(
                source,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )]
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

pub(crate) fn handle_symlinkat(
    source: ThreadIdentifier,
    request: SymbolicLinkAtRequest,
) -> Vec<Message> {
    match ::vfs::fd::vfs_symlinkat(&request.target, request.dirfd, &request.linkpath) {
        Ok(()) => {
            vec![SymbolicLinkAtResponse::build(
                source,
                0,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )]
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

pub(crate) fn handle_linkat(source: ThreadIdentifier, request: LinkAtRequest) -> Vec<Message> {
    match ::vfs::fd::vfs_linkat(
        request.olddirfd,
        &request.oldpath,
        request.newdirfd,
        &request.newpath,
        request.flags,
    ) {
        Ok(()) => {
            vec![LinkAtResponse::build(
                source,
                0,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )]
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

pub(crate) fn handle_readlinkat(
    source: ThreadIdentifier,
    request: ReadLinkAtRequest,
) -> Vec<Message> {
    // readlink is not supported by our VFS.
    ::syslog::trace!(
        "handle_readlinkat(): not supported (dirfd={}, path={:?})",
        request.dirfd,
        request.path
    );
    vec![build_error(source, ErrorCode::OperationNotSupported)]
}

pub(crate) fn handle_utimensat(
    source: ThreadIdentifier,
    request: UpdateFileAccessTimeAtRequest,
) -> Vec<Message> {
    match ::vfs::fd::vfs_utimensat(request.dirfd, &request.path, &request.times, request.flag) {
        Ok(()) => {
            vec![UpdateFileAccessTimeAtResponse::build(
                source,
                0,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )]
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

pub(crate) fn handle_fchownat(
    source: ThreadIdentifier,
    request: FileChownAtRequest,
) -> Vec<Message> {
    match ::vfs::fd::vfs_fchownat(
        request.dirfd,
        &request.path,
        request.owner,
        request.group,
        request.flag,
    ) {
        Ok(()) => {
            vec![FileChownAtResponse::build(
                source,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )]
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}

pub(crate) fn handle_fchmodat(
    source: ThreadIdentifier,
    request: FileChmodAtRequest,
) -> Vec<Message> {
    match ::vfs::fd::vfs_fchmodat(request.dirfd, &request.path, request.mode, request.flag) {
        Ok(()) => {
            vec![FileChmodAtResponse::build(
                source,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )]
        },
        Err(e) => vec![build_error(source, fat32_to_error_code(&e))],
    }
}
