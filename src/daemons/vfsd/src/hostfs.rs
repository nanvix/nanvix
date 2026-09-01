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
        RequestIdentifier,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::syscall::{
    message::SystemCallMessagePart,
    SystemCallMessageKind,
};
use ::vfs::path::ResolvedPath;

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
        MessageSender::VFSD,
        MessageReceiver::KERNEL,
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
    header: SystemCallMessageKind,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    if !long_msg::is_valid_long_message_size(data.len()) {
        return Err(::sys::error::ErrorCode::InvalidArgument);
    }

    let num_parts: u16 = data
        .len()
        .div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
        .try_into()
        .map_err(|_| ::sys::error::ErrorCode::InvalidArgument)?;

    for (part_number, chunk) in data.chunks(SystemCallMessagePart::PAYLOAD_SIZE).enumerate() {
        let mut payload = [0u8; SystemCallMessagePart::PAYLOAD_SIZE];
        payload[..chunk.len()].copy_from_slice(chunk);

        let mut message: Message = SystemCallMessagePart::build_request(
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
        RequestIdentifier::from_raw(u32::from_le_bytes(op_id.to_le_bytes())).write_to(&mut message);

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
    path: &ResolvedPath,
    flags: i32,
    mode: u32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path.as_str());
    let buf: alloc::vec::Vec<u8> =
        long_msg::serialize_long_open_request(op_id, flags, mode, relative.as_bytes())
            .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageKind::HostFsOpenRequestPart, op_id)
}

/// Sends a CLOSE request to hostfsd.
pub fn send_close_request(
    remote_fd: i32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let payload: [u8; Message::PAYLOAD_SIZE] = CloseRequest { fd: remote_fd }
        .serialize(SystemCallMessageKind::HostFsCloseRequest as u16, op_id);

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
    .serialize(SystemCallMessageKind::HostFsReadRequest as u16, op_id);

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
        .serialize(SystemCallMessageKind::HostFsWriteRequest as u16, op_id);

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
    .serialize(SystemCallMessageKind::HostFsLseekRequest as u16, op_id);

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
    .serialize(SystemCallMessageKind::HostFsTruncateRequest as u16, op_id);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a descriptor-based CHOWN request to hostfsd.
pub fn send_chown_request(
    remote_fd: i32,
    owner: u32,
    group: u32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let payload: [u8; Message::PAYLOAD_SIZE] = ChownRequest {
        fd: remote_fd,
        owner,
        group,
    }
    .serialize(SystemCallMessageKind::HostFsChownRequest as u16, op_id);

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
        .serialize(SystemCallMessageKind::HostFsFlushRequest as u16, op_id);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a descriptor-based timestamp update request to hostfsd.
pub fn send_update_times_request(
    remote_fd: i32,
    times: &[::sysapi::time::timespec; 2],
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let payload: [u8; Message::PAYLOAD_SIZE] = UpdateTimesRequest {
        fd: remote_fd,
        times: *times,
    }
    .serialize(SystemCallMessageKind::HostFsUpdateTimesRequest as u16, op_id);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a MKDIR request to hostfsd as a multi-part IKC message.
pub fn send_mkdir_request(
    path: &ResolvedPath,
    mode: u32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path.as_str());
    let buf: alloc::vec::Vec<u8> =
        long_msg::serialize_long_mkdir_request(op_id, mode, relative.as_bytes())
            .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageKind::HostFsMkdirRequestPart, op_id)
}

/// Sends an RMDIR request to hostfsd as a multi-part IKC message.
pub fn send_rmdir_request(
    path: &ResolvedPath,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path.as_str());
    let buf: alloc::vec::Vec<u8> =
        long_msg::serialize_long_rmdir_request(op_id, relative.as_bytes())
            .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageKind::HostFsRmdirRequestPart, op_id)
}

/// Sends an UNLINK request to hostfsd as a multi-part IKC message.
pub fn send_unlink_request(
    path: &ResolvedPath,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path.as_str());
    let buf: alloc::vec::Vec<u8> =
        long_msg::serialize_long_unlink_request(op_id, relative.as_bytes())
            .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageKind::HostFsUnlinkRequestPart, op_id)
}

/// Sends a STAT request to hostfsd (by remote FD).
pub fn send_stat_request(
    remote_fd: i32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let payload: [u8; Message::PAYLOAD_SIZE] = StatRequest { fd: remote_fd }
        .serialize(SystemCallMessageKind::HostFsStatRequest as u16, op_id);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a READDIR request to hostfsd for a single directory entry at `offset`.
///
/// hostfsd serves directory listings via offset-based iteration, returning one
/// [`ReadDirEntry`](::hostfs_api::ReadDirEntry) per request (`name_len == 0` signals
/// end-of-directory). vfsd drives this as an async sweep, issuing one request per
/// entry until the caller's requested count is satisfied or the directory is exhausted.
pub fn send_readdir_request(
    remote_fd: i32,
    offset: u32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let payload: [u8; Message::PAYLOAD_SIZE] = ::hostfs_api::ReadDirRequest {
        fd: remote_fd,
        _reserved: 0,
        offset,
    }
    .serialize(SystemCallMessageKind::HostFsReadDirRequest as u16, op_id);

    if send_request(&payload) {
        Ok(())
    } else {
        Err(::sys::error::ErrorCode::IoErr)
    }
}

/// Sends a RENAME request to hostfsd as a multi-part IKC message.
pub fn send_rename_request(
    old_path: &ResolvedPath,
    new_path: &ResolvedPath,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let old_relative: &str = strip_mount_prefix(old_path.as_str());
    let new_relative: &str = strip_mount_prefix(new_path.as_str());
    let buf: alloc::vec::Vec<u8> = long_msg::serialize_long_rename_request(
        op_id,
        old_relative.as_bytes(),
        new_relative.as_bytes(),
    )
    .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageKind::HostFsRenameRequestPart, op_id)
}

/// Sends a LINK request to hostfsd as a multi-part IKC message.
pub fn send_link_request(
    old_path: &ResolvedPath,
    new_path: &ResolvedPath,
    flags: i32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let old_relative: &str = strip_mount_prefix(old_path.as_str());
    let new_relative: &str = strip_mount_prefix(new_path.as_str());
    let buf: alloc::vec::Vec<u8> = long_msg::serialize_long_link_request(
        op_id,
        flags,
        old_relative.as_bytes(),
        new_relative.as_bytes(),
    )
    .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageKind::HostFsLinkRequestPart, op_id)
}

/// Sends a SYMLINK request to hostfsd as a multi-part IKC message.
///
/// `target` is the symlink target string (stored verbatim by the host) and `linkpath`
/// is the absolute guest path (under `/mnt`) where the symlink is to be created.
///
/// Unlike [`send_readlink_request`] and [`send_lstat_request`], this always uses the
/// multi-part wire format even for short payloads. Symlink has no inline single-message
/// request variant: the wire format carries two variable-length strings, which does
/// not fit cleanly in the inline payload budget. The cost of an extra IKC roundtrip is
/// acceptable since symlink creation is rare relative to readlink/lstat.
pub fn send_symlink_request(
    target: &str,
    linkpath: &ResolvedPath,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    // The target is opaque to vfsd and stored verbatim by the host; do not strip /mnt.
    let link_relative: &str = strip_mount_prefix(linkpath.as_str());
    let buf: alloc::vec::Vec<u8> = long_msg::serialize_long_symlink_request(
        op_id,
        target.as_bytes(),
        link_relative.as_bytes(),
    )
    .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageKind::HostFsSymlinkRequestPart, op_id)
}

/// Sends a READLINK request to hostfsd.
///
/// Uses the single-message inline form when the path fits within
/// [`MAX_INLINE_PATH_LEN`], and falls back to a multi-part request otherwise.
pub fn send_readlink_request(
    path: &ResolvedPath,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path.as_str());
    let path_bytes: &[u8] = relative.as_bytes();

    // Inline fast path: avoids the multi-part assembler when the path fits.
    if let Some(req) = ReadlinkRequest::from_path(path_bytes) {
        let payload: [u8; Message::PAYLOAD_SIZE] =
            req.serialize(SystemCallMessageKind::HostFsReadlinkRequest as u16, op_id);
        return if send_request(&payload) {
            Ok(())
        } else {
            Err(::sys::error::ErrorCode::IoErr)
        };
    }

    // Serialize the multi-part body via hostfs-api.
    let buf: alloc::vec::Vec<u8> = long_msg::serialize_long_readlink_request(op_id, path_bytes)
        .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageKind::HostFsReadlinkRequestPart, op_id)
}

/// Sends a path-based CHOWN request to hostfsd.
pub fn send_chownat_request(
    path: &ResolvedPath,
    owner: u32,
    group: u32,
    flags: i32,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path.as_str());
    let buf: alloc::vec::Vec<u8> =
        long_msg::serialize_long_chownat_request(op_id, owner, group, flags, relative.as_bytes())
            .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageKind::HostFsChownAtRequestPart, op_id)
}

/// Sends an LSTAT request to hostfsd.
///
/// Unlike [`send_stat_request`] (which takes a remote FD), this is a path-based
/// stat that does not follow the final symbolic link component. Uses the
/// single-message inline form when the path fits within [`MAX_INLINE_PATH_LEN`],
/// and falls back to a multi-part request otherwise.
pub fn send_lstat_request(
    path: &ResolvedPath,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path.as_str());
    let path_bytes: &[u8] = relative.as_bytes();

    // Inline fast path: avoids the multi-part assembler when the path fits.
    if let Some(req) = LstatRequest::from_path(path_bytes) {
        let payload: [u8; Message::PAYLOAD_SIZE] =
            req.serialize(SystemCallMessageKind::HostFsLstatRequest as u16, op_id);
        return if send_request(&payload) {
            Ok(())
        } else {
            Err(::sys::error::ErrorCode::IoErr)
        };
    }

    // Serialize the multi-part body via hostfs-api.
    let buf: alloc::vec::Vec<u8> = long_msg::serialize_long_lstat_request(op_id, path_bytes)
        .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageKind::HostFsLstatRequestPart, op_id)
}

/// Sends a path-based timestamp update request to hostfsd.
pub fn send_update_times_at_request(
    path: &ResolvedPath,
    flags: i32,
    times: &[::sysapi::time::timespec; 2],
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path.as_str());
    let buf: alloc::vec::Vec<u8> =
        long_msg::serialize_long_update_times_request(op_id, flags, times, relative.as_bytes())
            .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageKind::HostFsUpdateTimesAtRequestPart, op_id)
}

/// Sends a path-based *following* STAT request to hostfsd.
///
/// This is the following counterpart to [`send_lstat_request`]: it stats the path after
/// following any final symbolic link (the default `stat(2)`/`fstatat` semantics). It
/// reuses the lstat request wire format (path in, [`LstatResponse`] out) and is
/// distinguished only by the `HostFsPathStat*` headers. Uses the single-message inline
/// form when the path fits within [`MAX_INLINE_PATH_LEN`], and falls back to a
/// multi-part request otherwise.
pub fn send_pathstat_request(
    path: &ResolvedPath,
    op_id: OperationId,
) -> Result<(), ::sys::error::ErrorCode> {
    let relative: &str = strip_mount_prefix(path.as_str());
    let path_bytes: &[u8] = relative.as_bytes();

    // Inline fast path: avoids the multi-part assembler when the path fits.
    if let Some(req) = LstatRequest::from_path(path_bytes) {
        let payload: [u8; Message::PAYLOAD_SIZE] =
            req.serialize(SystemCallMessageKind::HostFsPathStatRequest as u16, op_id);
        return if send_request(&payload) {
            Ok(())
        } else {
            Err(::sys::error::ErrorCode::IoErr)
        };
    }

    // Serialize the multi-part body via hostfs-api (reuses the lstat wire format).
    let buf: alloc::vec::Vec<u8> = long_msg::serialize_long_lstat_request(op_id, path_bytes)
        .ok_or(::sys::error::ErrorCode::InvalidArgument)?;

    send_long_request(&buf, SystemCallMessageKind::HostFsPathStatRequestPart, op_id)
}
