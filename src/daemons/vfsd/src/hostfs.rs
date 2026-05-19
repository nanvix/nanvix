// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Host filesystem forwarding module for vfsd.
//!
//! This module handles forwarding VFS operations to the host filesystem daemon (hostfsd)
//! via IKC messages. When a file path resolves to the hostfs mount point (`/mnt`), vfsd
//! sends a request to hostfsd and a pending operation record is created. The main event
//! loop completes the operation when the IKC response arrives.
//!
//! # Protocol
//!
//! vfsd sends IKC messages with `hostfs-api` encoding to hostfsd. The response arrives
//! asynchronously in the main event loop as an IKC message and is dispatched via the
//! pending operation queue.

use ::hostfs_api::{
    OperationId,
    *,
};
use ::sys::{
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::ProcessIdentifier,
};
use ::syscall::SystemCallMessageHeader;
use core::sync::atomic::{
    AtomicBool,
    Ordering,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Mount path prefix that routes to the host filesystem.
pub const HOSTFS_MOUNT_PATH: &str = "/mnt";

//==================================================================================================
// State
//==================================================================================================

/// Whether hostfs is enabled (set during initialization when the host filesystem
/// daemon is available to serve file operations for paths under /mnt).
static HOSTFS_ENABLED: AtomicBool = AtomicBool::new(false);

/// Enables the hostfs forwarding subsystem.
pub fn enable() {
    HOSTFS_ENABLED.store(true, Ordering::Release);
}

/// Disables the hostfs forwarding subsystem.
pub fn disable() {
    HOSTFS_ENABLED.store(false, Ordering::Release);
}

/// Returns whether hostfs forwarding is enabled.
pub fn is_enabled() -> bool {
    HOSTFS_ENABLED.load(Ordering::Acquire)
}

//==================================================================================================
// Path Routing
//==================================================================================================

/// Returns `true` if the given path should be routed to hostfsd.
///
/// The exact path `/mnt` matches intentionally — it maps to the root of the mounted
/// directory via [`strip_mount_prefix`], allowing operations such as `open("/mnt")`
/// or `getdents("/mnt")` to enumerate the mount root.
pub fn is_hostfs_path(path: &str) -> bool {
    if !is_enabled() {
        return false;
    }
    path == HOSTFS_MOUNT_PATH || path.starts_with("/mnt/")
}

/// Strips the hostfs mount prefix from a path, returning the relative path.
///
/// E.g., `/mnt/foo/bar.txt` → `foo/bar.txt`
/// `/mnt` → `` (empty string, meaning root of mounted dir)
pub fn strip_mount_prefix(path: &str) -> &str {
    if path == HOSTFS_MOUNT_PATH {
        ""
    } else if let Some(rest) = path.strip_prefix("/mnt/") {
        rest
    } else {
        path
    }
}

//==================================================================================================
// IKC Request Sending
//==================================================================================================

/// Sends a hostfs IKC request to the host. Returns `true` on success.
///
/// This function does NOT wait for a response. The caller must register a pending
/// operation so that the main event loop can complete it when the IKC response arrives.
///
/// NOTE: `__kcall_send` may block if the kernel IKC send queue is full. In practice, the
/// queue depth is large enough that this behaves as non-blocking under normal load. Under
/// heavy write traffic, the caller (vfsd event loop) may stall briefly until a slot opens.
pub fn send_request(payload: &[u8; Message::PAYLOAD_SIZE]) -> bool {
    let message: Message = Message::new(
        MessageSender::from(ProcessIdentifier::VFSD),
        MessageReceiver::from(ProcessIdentifier::KERNEL),
        MessageType::Ikc,
        None,
        *payload,
    );

    if let Err(e) = ::sys::kcall::ipc::__kcall_send(&message) {
        ::syslog::error!("hostfs: failed to send IKC request (error={:?})", e);
        return false;
    }
    true
}

//==================================================================================================
// High-Level Operation Forwarding (non-blocking send)
//==================================================================================================

/// Sends an OPEN request to hostfsd.
pub fn send_open_request(
    path: &str,
    flags: i32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path);
    let path_bytes: &[u8] = relative.as_bytes();

    if path_bytes.len() > MAX_INLINE_PATH_LEN {
        ::syslog::error!("hostfs: path too long for inline message: {}", path);
        return Err(::sys::error::ErrorCode::InvalidArgument);
    }

    let mut req_path: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
    req_path[..path_bytes.len()].copy_from_slice(path_bytes);

    let req: OpenRequest = OpenRequest {
        flags,
        path_len: path_bytes.len() as u16,
        path: req_path,
    };

    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    set_header(&mut payload, SystemCallMessageHeader::HostFsOpenRequest as u16);
    set_op_id(&mut payload, op_id);
    req.encode(&mut payload);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a CLOSE request to hostfsd.
pub fn send_close_request(
    remote_fd: i32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let req: CloseRequest = CloseRequest { fd: remote_fd };
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    set_header(&mut payload, SystemCallMessageHeader::HostFsCloseRequest as u16);
    set_op_id(&mut payload, op_id);
    req.encode(&mut payload);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a READ request to hostfsd.
pub fn send_read_request(
    remote_fd: i32,
    count: usize,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let count: u32 = count.min(MAX_INLINE_READ_DATA) as u32;
    let req: ReadRequest = ReadRequest {
        fd: remote_fd,
        count,
        offset: -1, // Use current position.
    };

    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    set_header(&mut payload, SystemCallMessageHeader::HostFsReadRequest as u16);
    set_op_id(&mut payload, op_id);
    req.encode(&mut payload);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a WRITE request to hostfsd.
pub fn send_write_request(
    remote_fd: i32,
    buf: &[u8],
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let write_len: usize = buf.len().min(MAX_INLINE_WRITE_DATA);
    let mut data: [u8; MAX_INLINE_WRITE_DATA] = [0u8; MAX_INLINE_WRITE_DATA];
    data[..write_len].copy_from_slice(&buf[..write_len]);

    let req: WriteRequest = WriteRequest {
        fd: remote_fd,
        count: write_len as u32,
        offset: -1, // Use current position.
        data_len: write_len as u16,
        data,
    };

    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    set_header(&mut payload, SystemCallMessageHeader::HostFsWriteRequest as u16);
    set_op_id(&mut payload, op_id);
    req.encode(&mut payload);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a SEEK request to hostfsd.
pub fn send_lseek_request(
    remote_fd: i32,
    offset: i64,
    whence: i32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let req: LseekRequest = LseekRequest {
        fd: remote_fd,
        offset,
        whence,
    };

    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    set_header(&mut payload, SystemCallMessageHeader::HostFsLseekRequest as u16);
    set_op_id(&mut payload, op_id);
    req.encode(&mut payload);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a TRUNCATE request to hostfsd.
pub fn send_truncate_request(
    remote_fd: i32,
    length: i64,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let req: TruncateRequest = TruncateRequest {
        fd: remote_fd,
        length,
    };

    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    set_header(&mut payload, SystemCallMessageHeader::HostFsTruncateRequest as u16);
    set_op_id(&mut payload, op_id);
    req.encode(&mut payload);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a FLUSH (fsync) request to hostfsd.
pub fn send_flush_request(
    remote_fd: i32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let req: FlushRequest = FlushRequest { fd: remote_fd };

    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    set_header(&mut payload, SystemCallMessageHeader::HostFsFlushRequest as u16);
    set_op_id(&mut payload, op_id);
    req.encode(&mut payload);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a MKDIR request to hostfsd.
pub fn send_mkdir_request(
    path: &str,
    mode: u32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path);
    let path_bytes: &[u8] = relative.as_bytes();

    if path_bytes.len() > MAX_INLINE_PATH_LEN {
        return Err(::sys::error::ErrorCode::InvalidArgument);
    }

    let mut req_path: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
    req_path[..path_bytes.len()].copy_from_slice(path_bytes);

    let req: MkdirRequest = MkdirRequest {
        mode,
        path_len: path_bytes.len() as u16,
        path: req_path,
    };

    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    set_header(&mut payload, SystemCallMessageHeader::HostFsMkdirRequest as u16);
    set_op_id(&mut payload, op_id);
    req.encode(&mut payload);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends an RMDIR request to hostfsd.
pub fn send_rmdir_request(path: &str, op_id: OperationId) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path);
    let path_bytes: &[u8] = relative.as_bytes();

    if path_bytes.len() > MAX_INLINE_PATH_LEN {
        return Err(::sys::error::ErrorCode::InvalidArgument);
    }

    let mut req_path: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
    req_path[..path_bytes.len()].copy_from_slice(path_bytes);

    let req: RmdirRequest = RmdirRequest {
        path_len: path_bytes.len() as u16,
        path: req_path,
    };

    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    set_header(&mut payload, SystemCallMessageHeader::HostFsRmdirRequest as u16);
    set_op_id(&mut payload, op_id);
    req.encode(&mut payload);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends an UNLINK request to hostfsd.
pub fn send_unlink_request(path: &str, op_id: OperationId) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path);
    let path_bytes: &[u8] = relative.as_bytes();

    if path_bytes.len() > MAX_INLINE_PATH_LEN {
        return Err(::sys::error::ErrorCode::InvalidArgument);
    }

    let mut req_path: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
    req_path[..path_bytes.len()].copy_from_slice(path_bytes);

    let req: UnlinkRequest = UnlinkRequest {
        path_len: path_bytes.len() as u16,
        path: req_path,
    };

    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    set_header(&mut payload, SystemCallMessageHeader::HostFsUnlinkRequest as u16);
    set_op_id(&mut payload, op_id);
    req.encode(&mut payload);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a STAT request to hostfsd (by remote FD).
pub fn send_stat_request(
    remote_fd: i32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let req: StatRequest = StatRequest { fd: remote_fd };

    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    set_header(&mut payload, SystemCallMessageHeader::HostFsStatRequest as u16);
    set_op_id(&mut payload, op_id);
    req.encode(&mut payload);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a RENAME request to hostfsd.
pub fn send_rename_request(
    old_path: &str,
    new_path: &str,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let old_relative: &str = strip_mount_prefix(old_path);
    let new_relative: &str = strip_mount_prefix(new_path);
    let old_bytes: &[u8] = old_relative.as_bytes();
    let new_bytes: &[u8] = new_relative.as_bytes();

    if old_bytes.len() + new_bytes.len() > MAX_INLINE_PATH_LEN {
        return Err(::sys::error::ErrorCode::InvalidArgument);
    }

    let mut paths: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
    paths[..old_bytes.len()].copy_from_slice(old_bytes);
    paths[old_bytes.len()..old_bytes.len() + new_bytes.len()].copy_from_slice(new_bytes);

    let req: RenameRequest = RenameRequest {
        old_path_len: old_bytes.len() as u16,
        new_path_len: new_bytes.len() as u16,
        paths,
    };

    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    set_header(&mut payload, SystemCallMessageHeader::HostFsRenameRequest as u16);
    set_op_id(&mut payload, op_id);
    req.encode(&mut payload);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}
