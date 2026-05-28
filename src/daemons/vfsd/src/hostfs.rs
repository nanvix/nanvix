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
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::syscall::{
    message::SystemCallMessagePart,
    SystemCallMessageHeader,
};

extern crate alloc;

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

/// Sends a multi-part hostfs IKC request to the host.
///
/// The serialized request bytes are split into `SystemCallMessagePart::PAYLOAD_SIZE`
/// chunks and each chunk is sent as a separate IKC message with the given header.
/// The hostfsd assembler on the host side collects the parts and reconstructs the
/// full request.
fn send_long_request(
    data: &[u8],
    header: SystemCallMessageHeader,
) -> Result<(), ::sys::error::ErrorCode> {
    let num_parts: u16 = data
        .len()
        .div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        .try_into()
        .map_err(|_| ::sys::error::ErrorCode::InvalidArgument)?;

    for (part_number, chunk) in data.chunks(SystemCallMessagePart::PAYLOAD_SIZE).enumerate() {
        let mut payload = [0u8; SystemCallMessagePart::PAYLOAD_SIZE];
        payload[..chunk.len()].copy_from_slice(chunk);

        let message: Message = SystemCallMessagePart::build_request(
            ThreadIdentifier::VFSD,
            header,
            num_parts,
            part_number as u16,
            chunk.len() as u8,
            payload,
            ProcessIdentifier::KERNEL,
            MessageType::Ikc,
        )
        .map_err(|_| ::sys::error::ErrorCode::InvalidArgument)?;

        if let Err(e) = ::sys::kcall::ipc::__kcall_send(&message) {
            ::syslog::error!(
                "hostfs: failed to send long IKC request part {}/{} (error={:?})",
                part_number + 1,
                num_parts,
                e
            );
            return Err(::sys::error::ErrorCode::IoErr);
        }
    }

    Ok(())
}

//==================================================================================================
// High-Level Operation Forwarding (non-blocking send)
//==================================================================================================

/// Sends an OPEN request to hostfsd as a multi-part IKC message.
pub fn send_open_request(
    path: &str,
    flags: i32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path);
    let buf: alloc::vec::Vec<u8> =
        long_msg::serialize_long_open_request(op_id, flags, relative.as_bytes())
            .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageHeader::HostFsOpenRequestPart)
}

/// Sends a CLOSE request to hostfsd.
pub fn send_close_request(
    remote_fd: i32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let payload: [u8; Message::PAYLOAD_SIZE] = CloseRequest { fd: remote_fd }
        .serialize(SystemCallMessageHeader::HostFsCloseRequest as u16, op_id);

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
    let payload: [u8; Message::PAYLOAD_SIZE] = ReadRequest {
        fd: remote_fd,
        count,
        offset: -1, // Use current position.
    }
    .serialize(SystemCallMessageHeader::HostFsReadRequest as u16, op_id);

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
    // Offset -1 means use current file position.
    let payload: [u8; Message::PAYLOAD_SIZE] = WriteRequest::from_slice(remote_fd, -1, buf)
        .serialize(SystemCallMessageHeader::HostFsWriteRequest as u16, op_id);

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
    let payload: [u8; Message::PAYLOAD_SIZE] = LseekRequest {
        fd: remote_fd,
        offset,
        whence,
    }
    .serialize(SystemCallMessageHeader::HostFsLseekRequest as u16, op_id);

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
    let payload: [u8; Message::PAYLOAD_SIZE] = TruncateRequest {
        fd: remote_fd,
        length,
    }
    .serialize(SystemCallMessageHeader::HostFsTruncateRequest as u16, op_id);

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
    let payload: [u8; Message::PAYLOAD_SIZE] = FlushRequest { fd: remote_fd }
        .serialize(SystemCallMessageHeader::HostFsFlushRequest as u16, op_id);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a MKDIR request to hostfsd as a multi-part IKC message.
pub fn send_mkdir_request(
    path: &str,
    mode: u32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path);
    let buf: alloc::vec::Vec<u8> =
        long_msg::serialize_long_mkdir_request(op_id, mode, relative.as_bytes())
            .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageHeader::HostFsMkdirRequestPart)
}

/// Sends an RMDIR request to hostfsd as a multi-part IKC message.
pub fn send_rmdir_request(path: &str, op_id: OperationId) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path);
    let buf: alloc::vec::Vec<u8> =
        long_msg::serialize_long_rmdir_request(op_id, relative.as_bytes())
            .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageHeader::HostFsRmdirRequestPart)
}

/// Sends an UNLINK request to hostfsd as a multi-part IKC message.
pub fn send_unlink_request(path: &str, op_id: OperationId) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path);
    let buf: alloc::vec::Vec<u8> =
        long_msg::serialize_long_unlink_request(op_id, relative.as_bytes())
            .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageHeader::HostFsUnlinkRequestPart)
}

/// Sends a STAT request to hostfsd (by remote FD).
pub fn send_stat_request(
    remote_fd: i32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let payload: [u8; Message::PAYLOAD_SIZE] = StatRequest { fd: remote_fd }
        .serialize(SystemCallMessageHeader::HostFsStatRequest as u16, op_id);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a RENAME request to hostfsd as a multi-part IKC message.
pub fn send_rename_request(
    old_path: &str,
    new_path: &str,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let old_relative: &str = strip_mount_prefix(old_path);
    let new_relative: &str = strip_mount_prefix(new_path);
    let buf: alloc::vec::Vec<u8> = long_msg::serialize_long_rename_request(
        op_id,
        old_relative.as_bytes(),
        new_relative.as_bytes(),
    )
    .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageHeader::HostFsRenameRequestPart)
}
