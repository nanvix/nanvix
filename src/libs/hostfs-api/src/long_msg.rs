// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Multi-part (long) message wire format for hostfs operations that carry file paths.
//!
//! Operations that carry file paths (open, rename, unlink, mkdir, rmdir) may exceed the
//! 36-byte inline limit of single-message encoding. This module defines the wire format
//! for these requests that is split into [`SystemCallMessagePart`] chunks on the sending
//! side (vfsd) and reassembled on the receiving side (hostfsd).
//!
//! # Path Length Limit
//!
//! Path lengths are encoded as 16-bit unsigned integers, so each full path string
//! carried by these messages is limited to [`MAX_PATH_LEN`] (65 535) bytes. For
//! [`rename`-style operations](LongRenameRequest), this limit applies independently
//! to each of the old and new paths.
//!
//! # Wire Format (little-endian)
//!
//! - **Open**: `[op_id:4][flags:4][path_len:2][path:N]`
//! - **Unlink**: `[op_id:4][path_len:2][path:N]`
//! - **Rmdir**: `[op_id:4][path_len:2][path:N]`
//! - **Mkdir**: `[op_id:4][mode:4][path_len:2][path:N]`
//! - **Rename**: `[op_id:4][old_path_len:2][new_path_len:2][old_path:N][new_path:M]`
//! - **Link**: `[op_id:4][flags:4][old_path_len:2][new_path_len:2][old_path:N][new_path:M]`
//! - **Symlink**: `[op_id:4][target_len:2][linkpath_len:2][target:N][linkpath:M]`
//! - **Readlink**: `[op_id:4][path_len:2][path:N]`
//! - **Lstat**: `[op_id:4][path_len:2][path:N]`
//! - **ChownAt**: `[op_id:4][owner:4][group:4][flags:4][path_len:2][path:N]`
//! - **Update times**: `[op_id:4][flags:4][times:32][path_len:2][path:N]`
//!
//! # Long Response Format
//!
//! - **Readlink response**: `[op_id:4][status:4][target_len:2][target:N]`. Used by
//!   `HostFsReadlinkResponsePart` messages when the target string exceeds the inline
//!   response capacity. Errors are always reported via the single-message
//!   `HostFsReadlinkResponse` form; the multi-part response is only emitted for a
//!   successful `readlink` whose target does not fit inline.
//! - **ReadDir response**: `[op_id:4][is_dir:1][size:8][name_len:2][name:N]`. Used by
//!   `HostFsReadDirResponsePart` messages when a directory entry name exceeds the
//!   inline `ReadDirEntry` name capacity. The single-message `HostFsReadDirResponse`
//!   form (one inline entry, `name_len == 0` signalling end-of-directory) continues to
//!   carry short names and the end marker; the multi-part response is only emitted for
//!   a single entry whose name does not fit inline.

extern crate alloc;

use alloc::vec::Vec;
use core::mem;

use ::sys::ipc::Message;

use crate::OperationId;

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum length of a single path in the long-message wire format (u16::MAX).
pub const MAX_PATH_LEN: usize = u16::MAX as usize;

/// Header size for long open: op_id(4) + flags(4) + mode(4) + path_len(2) = 14.
pub const OPEN_HEADER_SIZE: usize = 14;

/// Header size for long unlink: op_id(4) + path_len(2) = 6
pub const UNLINK_HEADER_SIZE: usize = 6;

/// Header size for long rmdir: op_id(4) + path_len(2) = 6
pub const RMDIR_HEADER_SIZE: usize = 6;

/// Header size for long mkdir: op_id(4) + mode(4) + path_len(2) = 10
pub const MKDIR_HEADER_SIZE: usize = 10;

/// Header size for long rename: op_id(4) + old_path_len(2) + new_path_len(2) = 8
pub const RENAME_HEADER_SIZE: usize = 8;

/// Size of an operation identifier.
const SIZE_OF_OPERATION_ID: usize = mem::size_of::<u32>();
/// Size of request flags.
const SIZE_OF_FLAGS: usize = mem::size_of::<i32>();
/// Size of a path length.
const SIZE_OF_PATH_LEN: usize = mem::size_of::<u16>();
/// Offset of the link operation identifier.
const LINK_OFFSET_OF_OPERATION_ID: usize = 0;
/// Offset of the link flags.
const LINK_OFFSET_OF_FLAGS: usize = LINK_OFFSET_OF_OPERATION_ID + SIZE_OF_OPERATION_ID;
/// Offset of the old path length.
const LINK_OFFSET_OF_OLD_PATH_LEN: usize = LINK_OFFSET_OF_FLAGS + SIZE_OF_FLAGS;
/// Offset of the new path length.
const LINK_OFFSET_OF_NEW_PATH_LEN: usize = LINK_OFFSET_OF_OLD_PATH_LEN + SIZE_OF_PATH_LEN;
/// Header size for long link.
pub const LINK_HEADER_SIZE: usize = LINK_OFFSET_OF_NEW_PATH_LEN + SIZE_OF_PATH_LEN;

/// Header size for long symlink: op_id(4) + target_len(2) + linkpath_len(2) = 8
pub const SYMLINK_HEADER_SIZE: usize = 8;

/// Header size for long readlink: op_id(4) + path_len(2) = 6
pub const READLINK_HEADER_SIZE: usize = 6;

/// Header size for long lstat: op_id(4) + path_len(2) = 6
pub const LSTAT_HEADER_SIZE: usize = 6;

/// Header size for long chownat: op_id(4) + owner(4) + group(4) + flags(4) + path_len(2) = 18.
pub const CHOWNAT_HEADER_SIZE: usize = 18;

/// Header size for long timestamp update: op_id(4) + flags(4) + times(32) + path_len(2) = 42.
pub const UPDATE_TIMES_HEADER_SIZE: usize = 4 + 4 + 2 * ::sysapi::time::timespec::WIRE_SIZE + 2;

/// Header size for the long Readlink *response* body:
/// `op_id(4) + status(4) + target_len(2) = 10`.
pub const READLINK_RESPONSE_HEADER_SIZE: usize = 10;

/// Header size for the long ReadDir *response* body:
/// `op_id(4) + is_dir(1) + size(8) + name_len(2) = 15`.
pub const READDIR_RESPONSE_HEADER_SIZE: usize = 15;

/// Maximum number of body bytes that can be carried by a single multi-part
/// long-response message.
///
/// Derived from the wire layout enforced by [`build_long_response_part`]: the
/// outer `SystemCallMessage` header (2 bytes), request identifier (4 bytes), the inner
/// `SystemCallMessagePart` framing (`total_parts:2 + part_number:2 + payload_size:1` = 5 bytes),
/// and then the body chunk. Total framing overhead is 11 bytes, so the per-part chunk cap is
/// `Message::PAYLOAD_SIZE - 11`.
pub const LONG_RESPONSE_CHUNK_SIZE: usize = Message::PAYLOAD_SIZE - 11;

//==================================================================================================
// Long-response Deserialization (no_std-friendly)
//==================================================================================================

/// Result of deserializing the body of a long READLINK *response*.
///
/// The `target` field borrows directly from the input buffer; no allocation is
/// performed. This makes the helper usable from `no_std` callers (e.g., vfsd).
pub struct LongReadlinkResponse<'a> {
    /// Operation identifier echoed by hostfsd. Matches the request's `op_id`.
    pub op_id: crate::OperationId,
    /// Status code (`0` on success, negative `HOSTFS_ERR_*` value on failure).
    pub status: i32,
    /// Symbolic-link target bytes, exactly `target_len` long.
    pub target: &'a [u8],
}

/// Deserializes the body of a long READLINK response.
///
/// `bytes` is the assembled body in wire format
/// `[op_id:4][status:4][target_len:2][target:N]`. Returns `None` if the buffer is
/// shorter than the declared header, or if the declared `target_len` exceeds the
/// remainder of the buffer.
pub fn deserialize_long_readlink_response(bytes: &[u8]) -> Option<LongReadlinkResponse<'_>> {
    if bytes.len() < READLINK_RESPONSE_HEADER_SIZE {
        return None;
    }
    let op_id: crate::OperationId =
        crate::OperationId::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let status: i32 = i32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let target_len: usize = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    let target_start: usize = READLINK_RESPONSE_HEADER_SIZE;
    let target_end: usize = target_start.checked_add(target_len)?;
    if bytes.len() < target_end {
        return None;
    }
    Some(LongReadlinkResponse {
        op_id,
        status,
        target: &bytes[target_start..target_end],
    })
}

/// Builds a single multi-part long-response message payload.
///
/// Wire layout (within `Message::PAYLOAD_SIZE`):
///
/// ```text
/// bytes  0..2   : outer SystemCallMessageKind discriminant (u16, native-endian)
/// bytes  2..6   : request identifier / hostfs op_id (u32, native-endian)
/// bytes  6..8   : total_parts (u16 LE)
/// bytes  8..10  : part_number (u16 LE)
/// byte   10     : payload_size (u8)
/// bytes 11..    : payload chunk (`chunk.len()` bytes)
/// ```
///
/// This mirrors the existing multi-part *request* wire format: the inner
/// `SystemCallMessagePart` fields (`total_parts`, `part_number`, `payload_size`,
/// payload) are encoded into the outer `SystemCallMessage` payload so the receiver
/// can decode it with the standard
/// `SystemCallMessage::try_from_bytes` → `SystemCallMessagePart::from_bytes` path.
///
/// `kind_value` is the outer `SystemCallMessageKind` discriminant (as a raw
/// `u16`) of the *Part* response variant being emitted (e.g.
/// `SystemCallMessageKind::HostFsReadlinkResponsePart as u16`). It is taken as a
/// raw value so this crate does not need to depend on the `syscall` crate.
///
/// The request `op_id` is echoed as the outer request identifier and remains in the first four
/// bytes of the assembled body for compatibility with the hostfs response assembler.
///
/// `chunk` must not exceed [`LONG_RESPONSE_CHUNK_SIZE`]. The byte at offset 10 encodes the chunk
/// length, so it must also fit in a `u8`.
///
/// Returns `None` if `chunk` does not fit in the message payload.
pub fn build_long_response_part(
    kind_value: u16,
    op_id: OperationId,
    total_parts: u16,
    part_number: u16,
    chunk: &[u8],
) -> Option<[u8; Message::PAYLOAD_SIZE]> {
    // The chunk length is written into a single byte at offset 10 and must also fit
    // within the outer Message payload after the 11-byte framing header.
    if chunk.len() > LONG_RESPONSE_CHUNK_SIZE || chunk.len() > u8::MAX as usize {
        return None;
    }

    let mut out: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    // Outer SystemCallMessage header.
    out[0..2].copy_from_slice(&kind_value.to_ne_bytes());
    // Outer request identifier, using the existing hostfs operation identifier.
    out[2..6].copy_from_slice(&op_id.raw().to_ne_bytes());
    // Inner SystemCallMessagePart fields, encoded little-endian to match
    // `SystemCallMessagePart::from_bytes` on the receiving side. Offsets are
    // relative to the outer Message::PAYLOAD because SystemCallMessage::PAYLOAD starts at byte 6.
    out[6..8].copy_from_slice(&total_parts.to_le_bytes());
    out[8..10].copy_from_slice(&part_number.to_le_bytes());
    out[10] = chunk.len() as u8;
    out[11..11 + chunk.len()].copy_from_slice(chunk);
    Some(out)
}

/// Maximum number of chunks a single long response stream may contain.
///
/// `total_parts` and `part_number` are encoded as `u16` in the
/// `SystemCallMessagePart` framing, so the assembled body cannot be split into
/// more than `u16::MAX` parts without producing a malformed (wrapped) stream.
pub const MAX_LONG_RESPONSE_PARTS: usize = u16::MAX as usize;

/// Splits an assembled long-response `body` into a sequence of framed multi-part
/// payloads, each stamped with `kind_value` as the outer
/// `SystemCallMessageKind` discriminant.
///
/// `body` is chunked into [`LONG_RESPONSE_CHUNK_SIZE`]-byte slices; each slice is
/// wrapped with [`build_long_response_part`] using `total_parts =
/// ceil(body.len() / LONG_RESPONSE_CHUNK_SIZE)` and a sequential `part_number`.
///
/// Returns `None` if `body` would require more than [`MAX_LONG_RESPONSE_PARTS`]
/// chunks (i.e. its chunk count does not fit in the `u16` wire-format field),
/// to avoid producing a malformed stream with wrapped `total_parts`/`part_number`
/// values. Otherwise returns a `Vec` that is non-empty for a non-empty `body`;
/// callers are responsible for queueing/distributing the resulting parts (e.g.
/// writing the first into a response slot and pushing the rest onto a tail queue).
pub fn chunk_long_response(
    kind_value: u16,
    op_id: OperationId,
    body: &[u8],
) -> Option<Vec<[u8; Message::PAYLOAD_SIZE]>> {
    let chunk_size: usize = LONG_RESPONSE_CHUNK_SIZE;
    let total_parts_usize: usize = body.len().div_ceil(chunk_size);
    if total_parts_usize > MAX_LONG_RESPONSE_PARTS {
        return None;
    }
    let total_parts: u16 = total_parts_usize as u16;
    let mut parts: Vec<[u8; Message::PAYLOAD_SIZE]> = Vec::with_capacity(total_parts_usize);
    for (part_number, chunk) in body.chunks(chunk_size).enumerate() {
        parts.push(build_long_response_part(
            kind_value,
            op_id,
            total_parts,
            part_number as u16,
            chunk,
        )?);
    }
    Some(parts)
}

/// Serializes the body of a long READLINK *response*.
///
/// Wire format: `[op_id:4][status:4][target_len:2][target:N]` (see
/// [`READLINK_RESPONSE_HEADER_SIZE`]). This is the inverse of
/// [`deserialize_long_readlink_response`]. Returns `None` if `target` is longer
/// than [`MAX_PATH_LEN`].
pub fn serialize_long_readlink_response(
    op_id: OperationId,
    status: i32,
    target: &[u8],
) -> Option<Vec<u8>> {
    let target_len: u16 = u16::try_from(target.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(READLINK_RESPONSE_HEADER_SIZE + target.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.extend_from_slice(&status.to_le_bytes());
    buf.extend_from_slice(&target_len.to_le_bytes());
    buf.extend_from_slice(target);
    Some(buf)
}

/// Result of deserializing the body of a long READDIR *response*.
///
/// The `name` field borrows directly from the input buffer; no allocation is
/// performed, making the helper usable from `no_std` callers (e.g., vfsd).
pub struct LongReaddirResponse<'a> {
    /// Operation identifier echoed by hostfsd. Matches the request's `op_id`.
    pub op_id: crate::OperationId,
    /// Whether the entry is a directory.
    pub is_dir: bool,
    /// File size in bytes.
    pub size: u64,
    /// Entry name bytes, exactly `name_len` long.
    pub name: &'a [u8],
}

/// Serializes the body of a long READDIR *response*.
///
/// Wire format: `[op_id:4][is_dir:1][size:8][name_len:2][name:N]` (see
/// [`READDIR_RESPONSE_HEADER_SIZE`]). This is the inverse of
/// [`deserialize_long_readdir_response`]. Returns `None` if `name` is longer than
/// [`MAX_PATH_LEN`] (the `u16` name-length field cannot represent it).
pub fn serialize_long_readdir_response(
    op_id: OperationId,
    is_dir: bool,
    size: u64,
    name: &[u8],
) -> Option<Vec<u8>> {
    let name_len: u16 = u16::try_from(name.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(READDIR_RESPONSE_HEADER_SIZE + name.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.push(if is_dir { 1 } else { 0 });
    buf.extend_from_slice(&size.to_le_bytes());
    buf.extend_from_slice(&name_len.to_le_bytes());
    buf.extend_from_slice(name);
    Some(buf)
}

/// Deserializes the body of a long READDIR response.
///
/// `bytes` is the assembled body in wire format
/// `[op_id:4][is_dir:1][size:8][name_len:2][name:N]`. Returns `None` if the buffer is
/// shorter than the declared header, or if the declared `name_len` exceeds the
/// remainder of the buffer.
pub fn deserialize_long_readdir_response(bytes: &[u8]) -> Option<LongReaddirResponse<'_>> {
    if bytes.len() < READDIR_RESPONSE_HEADER_SIZE {
        return None;
    }
    let op_id: crate::OperationId =
        crate::OperationId::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let is_dir: bool = bytes[4] != 0;
    let size: u64 = u64::from_le_bytes([
        bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12],
    ]);
    let name_len: usize = u16::from_le_bytes([bytes[13], bytes[14]]) as usize;
    let name_start: usize = READDIR_RESPONSE_HEADER_SIZE;
    let name_end: usize = name_start.checked_add(name_len)?;
    if bytes.len() < name_end {
        return None;
    }
    Some(LongReaddirResponse {
        op_id,
        is_dir,
        size,
        name: &bytes[name_start..name_end],
    })
}

//==================================================================================================
// Deserialization (hostfsd, std)
//==================================================================================================

/// Result of deserializing a long OPEN request.
#[cfg(feature = "std")]
pub struct LongOpenRequest {
    pub op_id: OperationId,
    pub flags: i32,
    pub mode: u32,
    pub path: std::string::String,
}

/// Deserializes a long OPEN request from assembled bytes.
#[cfg(feature = "std")]
pub fn deserialize_long_open(bytes: &[u8]) -> Option<LongOpenRequest> {
    if bytes.len() < OPEN_HEADER_SIZE {
        return None;
    }
    let op_id = OperationId::new(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
    let flags = i32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let mode = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    let path_len = u16::from_le_bytes([bytes[12], bytes[13]]) as usize;
    if bytes.len() < OPEN_HEADER_SIZE + path_len {
        return None;
    }
    let path = std::string::String::from_utf8(
        bytes[OPEN_HEADER_SIZE..OPEN_HEADER_SIZE + path_len].to_vec(),
    )
    .ok()?;
    Some(LongOpenRequest {
        op_id,
        flags,
        mode,
        path,
    })
}

/// Result of deserializing a long UNLINK request.
#[cfg(feature = "std")]
pub struct LongUnlinkRequest {
    pub op_id: OperationId,
    pub path: std::string::String,
}

/// Deserializes a long UNLINK request from assembled bytes.
#[cfg(feature = "std")]
pub fn deserialize_long_unlink(bytes: &[u8]) -> Option<LongUnlinkRequest> {
    if bytes.len() < UNLINK_HEADER_SIZE {
        return None;
    }
    let op_id = OperationId::new(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
    let path_len = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    if bytes.len() < UNLINK_HEADER_SIZE + path_len {
        return None;
    }
    let path = std::string::String::from_utf8(
        bytes[UNLINK_HEADER_SIZE..UNLINK_HEADER_SIZE + path_len].to_vec(),
    )
    .ok()?;
    Some(LongUnlinkRequest { op_id, path })
}

/// Result of deserializing a long RMDIR request.
#[cfg(feature = "std")]
pub struct LongRmdirRequest {
    pub op_id: OperationId,
    pub path: std::string::String,
}

/// Deserializes a long RMDIR request from assembled bytes.
#[cfg(feature = "std")]
pub fn deserialize_long_rmdir(bytes: &[u8]) -> Option<LongRmdirRequest> {
    if bytes.len() < RMDIR_HEADER_SIZE {
        return None;
    }
    let op_id = OperationId::new(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
    let path_len = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    if bytes.len() < RMDIR_HEADER_SIZE + path_len {
        return None;
    }
    let path = std::string::String::from_utf8(
        bytes[RMDIR_HEADER_SIZE..RMDIR_HEADER_SIZE + path_len].to_vec(),
    )
    .ok()?;
    Some(LongRmdirRequest { op_id, path })
}

/// Result of deserializing a long MKDIR request.
#[cfg(feature = "std")]
pub struct LongMkdirRequest {
    pub op_id: OperationId,
    pub mode: u32,
    pub path: std::string::String,
}

/// Deserializes a long MKDIR request from assembled bytes.
#[cfg(feature = "std")]
pub fn deserialize_long_mkdir(bytes: &[u8]) -> Option<LongMkdirRequest> {
    if bytes.len() < MKDIR_HEADER_SIZE {
        return None;
    }
    let op_id = OperationId::new(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
    let mode = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let path_len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    if bytes.len() < MKDIR_HEADER_SIZE + path_len {
        return None;
    }
    let path = std::string::String::from_utf8(
        bytes[MKDIR_HEADER_SIZE..MKDIR_HEADER_SIZE + path_len].to_vec(),
    )
    .ok()?;
    Some(LongMkdirRequest { op_id, mode, path })
}

/// Result of deserializing a long RENAME request.
#[cfg(feature = "std")]
pub struct LongRenameRequest {
    pub op_id: OperationId,
    pub old_path: std::string::String,
    pub new_path: std::string::String,
}

/// Deserializes a long RENAME request from assembled bytes.
#[cfg(feature = "std")]
pub fn deserialize_long_rename(bytes: &[u8]) -> Option<LongRenameRequest> {
    if bytes.len() < RENAME_HEADER_SIZE {
        return None;
    }
    let op_id = OperationId::new(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
    let old_path_len = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    let new_path_len = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    if bytes.len() < RENAME_HEADER_SIZE + old_path_len + new_path_len {
        return None;
    }
    let old_start = RENAME_HEADER_SIZE;
    let new_start = old_start + old_path_len;
    let old_path =
        std::string::String::from_utf8(bytes[old_start..old_start + old_path_len].to_vec()).ok()?;
    let new_path =
        std::string::String::from_utf8(bytes[new_start..new_start + new_path_len].to_vec()).ok()?;
    Some(LongRenameRequest {
        op_id,
        old_path,
        new_path,
    })
}

/// Result of deserializing a long LINK request.
#[cfg(feature = "std")]
pub struct LongLinkRequest {
    pub op_id: OperationId,
    pub flags: i32,
    pub old_path: std::string::String,
    pub new_path: std::string::String,
}

/// Deserializes a long LINK request from assembled bytes.
#[cfg(feature = "std")]
pub fn deserialize_long_link(bytes: &[u8]) -> Option<LongLinkRequest> {
    if bytes.len() < LINK_HEADER_SIZE {
        return None;
    }
    let op_id = OperationId::new(u32::from_le_bytes(
        bytes[LINK_OFFSET_OF_OPERATION_ID..LINK_OFFSET_OF_FLAGS]
            .try_into()
            .ok()?,
    ));
    let flags = i32::from_le_bytes(
        bytes[LINK_OFFSET_OF_FLAGS..LINK_OFFSET_OF_OLD_PATH_LEN]
            .try_into()
            .ok()?,
    );
    let old_path_len = u16::from_le_bytes(
        bytes[LINK_OFFSET_OF_OLD_PATH_LEN..LINK_OFFSET_OF_NEW_PATH_LEN]
            .try_into()
            .ok()?,
    ) as usize;
    let new_path_len = u16::from_le_bytes(
        bytes[LINK_OFFSET_OF_NEW_PATH_LEN..LINK_HEADER_SIZE]
            .try_into()
            .ok()?,
    ) as usize;
    if bytes.len() < LINK_HEADER_SIZE + old_path_len + new_path_len {
        return None;
    }
    let old_start = LINK_HEADER_SIZE;
    let new_start = old_start + old_path_len;
    let old_path =
        std::string::String::from_utf8(bytes[old_start..old_start + old_path_len].to_vec()).ok()?;
    let new_path =
        std::string::String::from_utf8(bytes[new_start..new_start + new_path_len].to_vec()).ok()?;
    Some(LongLinkRequest {
        op_id,
        flags,
        old_path,
        new_path,
    })
}

/// Result of deserializing a long SYMLINK request.
///
/// `target` is the textual target stored in the symbolic link (interpreted verbatim;
/// it is not validated to be a sandbox-relative path). `linkpath` is the path of the
/// link to be created.
#[cfg(feature = "std")]
pub struct LongSymlinkRequest {
    pub op_id: OperationId,
    pub target: std::string::String,
    pub linkpath: std::string::String,
}

/// Deserializes a long SYMLINK request from assembled bytes.
#[cfg(feature = "std")]
pub fn deserialize_long_symlink(bytes: &[u8]) -> Option<LongSymlinkRequest> {
    if bytes.len() < SYMLINK_HEADER_SIZE {
        return None;
    }
    let op_id = OperationId::new(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
    let target_len = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    let linkpath_len = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    if bytes.len() < SYMLINK_HEADER_SIZE + target_len + linkpath_len {
        return None;
    }
    let target_start = SYMLINK_HEADER_SIZE;
    let linkpath_start = target_start + target_len;
    let target =
        std::string::String::from_utf8(bytes[target_start..target_start + target_len].to_vec())
            .ok()?;
    let linkpath = std::string::String::from_utf8(
        bytes[linkpath_start..linkpath_start + linkpath_len].to_vec(),
    )
    .ok()?;
    Some(LongSymlinkRequest {
        op_id,
        target,
        linkpath,
    })
}

/// Result of deserializing a long READLINK request.
#[cfg(feature = "std")]
pub struct LongReadlinkRequest {
    pub op_id: OperationId,
    pub path: std::string::String,
}

/// Deserializes a long READLINK request from assembled bytes.
#[cfg(feature = "std")]
pub fn deserialize_long_readlink(bytes: &[u8]) -> Option<LongReadlinkRequest> {
    if bytes.len() < READLINK_HEADER_SIZE {
        return None;
    }
    let op_id = OperationId::new(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
    let path_len = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    if bytes.len() < READLINK_HEADER_SIZE + path_len {
        return None;
    }
    let path = std::string::String::from_utf8(
        bytes[READLINK_HEADER_SIZE..READLINK_HEADER_SIZE + path_len].to_vec(),
    )
    .ok()?;
    Some(LongReadlinkRequest { op_id, path })
}

/// Result of deserializing a long LSTAT request.
#[cfg(feature = "std")]
pub struct LongLstatRequest {
    pub op_id: OperationId,
    pub path: std::string::String,
}

/// Deserializes a long LSTAT request from assembled bytes.
#[cfg(feature = "std")]
pub fn deserialize_long_lstat(bytes: &[u8]) -> Option<LongLstatRequest> {
    if bytes.len() < LSTAT_HEADER_SIZE {
        return None;
    }
    let op_id = OperationId::new(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
    let path_len = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    if bytes.len() < LSTAT_HEADER_SIZE + path_len {
        return None;
    }
    let path = std::string::String::from_utf8(
        bytes[LSTAT_HEADER_SIZE..LSTAT_HEADER_SIZE + path_len].to_vec(),
    )
    .ok()?;
    Some(LongLstatRequest { op_id, path })
}

/// Result of deserializing a long CHOWNAT request.
#[cfg(feature = "std")]
pub struct LongChownAtRequest {
    pub op_id: OperationId,
    pub owner: u32,
    pub group: u32,
    pub flags: i32,
    pub path: std::string::String,
}

/// Deserializes a long CHOWNAT request from assembled bytes.
#[cfg(feature = "std")]
pub fn deserialize_long_chownat(bytes: &[u8]) -> Option<LongChownAtRequest> {
    if bytes.len() < CHOWNAT_HEADER_SIZE {
        return None;
    }
    let op_id = OperationId::new(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
    let owner = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let group = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    let flags = i32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    let path_len = u16::from_le_bytes([bytes[16], bytes[17]]) as usize;
    if bytes.len() < CHOWNAT_HEADER_SIZE + path_len {
        return None;
    }
    let path = std::string::String::from_utf8(
        bytes[CHOWNAT_HEADER_SIZE..CHOWNAT_HEADER_SIZE + path_len].to_vec(),
    )
    .ok()?;
    Some(LongChownAtRequest {
        op_id,
        owner,
        group,
        flags,
        path,
    })
}

/// Result of deserializing a long path-based timestamp update request.
#[cfg(feature = "std")]
pub struct LongUpdateTimesRequest {
    pub op_id: OperationId,
    pub flags: i32,
    pub times: [::sysapi::time::timespec; 2],
    pub path: std::string::String,
}

/// Deserializes a long path-based timestamp update request.
#[cfg(feature = "std")]
pub fn deserialize_long_update_times(bytes: &[u8]) -> Option<LongUpdateTimesRequest> {
    use ::sysapi::time::timespec;

    if bytes.len() < UPDATE_TIMES_HEADER_SIZE {
        return None;
    }
    let op_id = OperationId::new(u32::from_le_bytes(bytes[0..4].try_into().ok()?));
    let flags = i32::from_le_bytes(bytes[4..8].try_into().ok()?);
    let times_start: usize = 8;
    let times = [
        timespec::try_from_bytes(&bytes[times_start..times_start + timespec::WIRE_SIZE]).ok()?,
        timespec::try_from_bytes(
            &bytes[times_start + timespec::WIRE_SIZE..times_start + 2 * timespec::WIRE_SIZE],
        )
        .ok()?,
    ];
    let path_len_start: usize = times_start + 2 * timespec::WIRE_SIZE;
    let path_len =
        u16::from_le_bytes(bytes[path_len_start..path_len_start + 2].try_into().ok()?) as usize;
    let path_start: usize = UPDATE_TIMES_HEADER_SIZE;
    let path_end: usize = path_start.checked_add(path_len)?;
    let path = std::string::String::from_utf8(bytes.get(path_start..path_end)?.to_vec()).ok()?;
    Some(LongUpdateTimesRequest {
        op_id,
        flags,
        times,
        path,
    })
}

//==================================================================================================
// Serialization (vfsd, no_std + alloc)
//==================================================================================================

/// Serializes the body of a long OPEN request.
///
/// Wire format: `[op_id:4][flags:4][mode:4][path_len:2][path:N]`. Returns `None` if `path`
/// is longer than [`MAX_PATH_LEN`].
pub fn serialize_long_open_request(
    op_id: OperationId,
    flags: i32,
    mode: u32,
    path: &[u8],
) -> Option<Vec<u8>> {
    let path_len: u16 = u16::try_from(path.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(OPEN_HEADER_SIZE + path.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.extend_from_slice(&flags.to_le_bytes());
    buf.extend_from_slice(&mode.to_le_bytes());
    buf.extend_from_slice(&path_len.to_le_bytes());
    buf.extend_from_slice(path);
    Some(buf)
}

/// Serializes the body of a long UNLINK request.
///
/// Wire format: `[op_id:4][path_len:2][path:N]`. Returns `None` if `path` is
/// longer than [`MAX_PATH_LEN`].
pub fn serialize_long_unlink_request(op_id: OperationId, path: &[u8]) -> Option<Vec<u8>> {
    let path_len: u16 = u16::try_from(path.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(UNLINK_HEADER_SIZE + path.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.extend_from_slice(&path_len.to_le_bytes());
    buf.extend_from_slice(path);
    Some(buf)
}

/// Serializes the body of a long RMDIR request.
///
/// Wire format: `[op_id:4][path_len:2][path:N]`. Returns `None` if `path` is
/// longer than [`MAX_PATH_LEN`].
pub fn serialize_long_rmdir_request(op_id: OperationId, path: &[u8]) -> Option<Vec<u8>> {
    let path_len: u16 = u16::try_from(path.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(RMDIR_HEADER_SIZE + path.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.extend_from_slice(&path_len.to_le_bytes());
    buf.extend_from_slice(path);
    Some(buf)
}

/// Serializes the body of a long MKDIR request.
///
/// Wire format: `[op_id:4][mode:4][path_len:2][path:N]`. Returns `None` if `path`
/// is longer than [`MAX_PATH_LEN`].
pub fn serialize_long_mkdir_request(op_id: OperationId, mode: u32, path: &[u8]) -> Option<Vec<u8>> {
    let path_len: u16 = u16::try_from(path.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(MKDIR_HEADER_SIZE + path.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.extend_from_slice(&mode.to_le_bytes());
    buf.extend_from_slice(&path_len.to_le_bytes());
    buf.extend_from_slice(path);
    Some(buf)
}

/// Serializes the body of a long RENAME request.
///
/// Wire format: `[op_id:4][old_path_len:2][new_path_len:2][old_path:N][new_path:M]`.
/// Returns `None` if either path is longer than [`MAX_PATH_LEN`].
pub fn serialize_long_rename_request(
    op_id: OperationId,
    old_path: &[u8],
    new_path: &[u8],
) -> Option<Vec<u8>> {
    let old_path_len: u16 = u16::try_from(old_path.len()).ok()?;
    let new_path_len: u16 = u16::try_from(new_path.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(RENAME_HEADER_SIZE + old_path.len() + new_path.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.extend_from_slice(&old_path_len.to_le_bytes());
    buf.extend_from_slice(&new_path_len.to_le_bytes());
    buf.extend_from_slice(old_path);
    buf.extend_from_slice(new_path);
    Some(buf)
}

/// Serializes the body of a long LINK request.
///
/// Wire format: `[op_id:4][flags:4][old_path_len:2][new_path_len:2][old_path:N][new_path:M]`.
/// Returns `None` if either path is longer than [`MAX_PATH_LEN`].
pub fn serialize_long_link_request(
    op_id: OperationId,
    flags: i32,
    old_path: &[u8],
    new_path: &[u8],
) -> Option<Vec<u8>> {
    let old_path_len: u16 = u16::try_from(old_path.len()).ok()?;
    let new_path_len: u16 = u16::try_from(new_path.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(LINK_HEADER_SIZE + old_path.len() + new_path.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.extend_from_slice(&flags.to_le_bytes());
    buf.extend_from_slice(&old_path_len.to_le_bytes());
    buf.extend_from_slice(&new_path_len.to_le_bytes());
    buf.extend_from_slice(old_path);
    buf.extend_from_slice(new_path);
    Some(buf)
}

/// Serializes the body of a long SYMLINK request.
///
/// Wire format: `[op_id:4][target_len:2][linkpath_len:2][target:N][linkpath:M]`.
/// Returns `None` if either string is longer than [`MAX_PATH_LEN`].
pub fn serialize_long_symlink_request(
    op_id: OperationId,
    target: &[u8],
    linkpath: &[u8],
) -> Option<Vec<u8>> {
    let target_len: u16 = u16::try_from(target.len()).ok()?;
    let link_len: u16 = u16::try_from(linkpath.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(SYMLINK_HEADER_SIZE + target.len() + linkpath.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.extend_from_slice(&target_len.to_le_bytes());
    buf.extend_from_slice(&link_len.to_le_bytes());
    buf.extend_from_slice(target);
    buf.extend_from_slice(linkpath);
    Some(buf)
}

/// Serializes the body of a long READLINK request.
///
/// Wire format: `[op_id:4][path_len:2][path:N]`. Returns `None` if `path` is
/// longer than [`MAX_PATH_LEN`].
pub fn serialize_long_readlink_request(op_id: OperationId, path: &[u8]) -> Option<Vec<u8>> {
    let path_len: u16 = u16::try_from(path.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(READLINK_HEADER_SIZE + path.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.extend_from_slice(&path_len.to_le_bytes());
    buf.extend_from_slice(path);
    Some(buf)
}

/// Serializes the body of a long LSTAT request.
///
/// Wire format: `[op_id:4][path_len:2][path:N]`. Returns `None` if `path` is
/// longer than [`MAX_PATH_LEN`].
pub fn serialize_long_lstat_request(op_id: OperationId, path: &[u8]) -> Option<Vec<u8>> {
    let path_len: u16 = u16::try_from(path.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(LSTAT_HEADER_SIZE + path.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.extend_from_slice(&path_len.to_le_bytes());
    buf.extend_from_slice(path);
    Some(buf)
}

/// Serializes the body of a long CHOWNAT request.
///
/// Wire format: `[op_id:4][owner:4][group:4][flags:4][path_len:2][path:N]`.
pub fn serialize_long_chownat_request(
    op_id: OperationId,
    owner: u32,
    group: u32,
    flags: i32,
    path: &[u8],
) -> Option<Vec<u8>> {
    let path_len: u16 = u16::try_from(path.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(CHOWNAT_HEADER_SIZE + path.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.extend_from_slice(&owner.to_le_bytes());
    buf.extend_from_slice(&group.to_le_bytes());
    buf.extend_from_slice(&flags.to_le_bytes());
    buf.extend_from_slice(&path_len.to_le_bytes());
    buf.extend_from_slice(path);
    Some(buf)
}

/// Serializes a long path-based timestamp update request.
pub fn serialize_long_update_times_request(
    op_id: OperationId,
    flags: i32,
    times: &[::sysapi::time::timespec; 2],
    path: &[u8],
) -> Option<Vec<u8>> {
    let path_len: u16 = u16::try_from(path.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(UPDATE_TIMES_HEADER_SIZE + path.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.extend_from_slice(&flags.to_le_bytes());
    buf.extend_from_slice(&times[0].to_bytes());
    buf.extend_from_slice(&times[1].to_bytes());
    buf.extend_from_slice(&path_len.to_le_bytes());
    buf.extend_from_slice(path);
    Some(buf)
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "std")]
    fn long_link_round_trip() {
        let op_id: OperationId = OperationId::new(11);
        let bytes: Vec<u8> = serialize_long_link_request(op_id, 0x400, b"old/path", b"new/path")
            .expect("link request should serialize");
        let request: LongLinkRequest =
            deserialize_long_link(&bytes).expect("link request should deserialize");

        assert_eq!(request.op_id, op_id);
        assert_eq!(request.flags, 0x400);
        assert_eq!(request.old_path, "old/path");
        assert_eq!(request.new_path, "new/path");
    }

    #[test]
    fn long_response_part_enforces_chunk_limit() {
        let chunk: [u8; LONG_RESPONSE_CHUNK_SIZE] = [0x5a; LONG_RESPONSE_CHUNK_SIZE];
        let part: [u8; Message::PAYLOAD_SIZE] =
            build_long_response_part(7, OperationId::new(11), 1, 0, &chunk)
                .expect("maximum-sized chunk should fit");
        assert_eq!(part[10] as usize, LONG_RESPONSE_CHUNK_SIZE);
        assert_eq!(&part[11..], &chunk);

        let oversized: [u8; LONG_RESPONSE_CHUNK_SIZE + 1] = [0; LONG_RESPONSE_CHUNK_SIZE + 1];
        assert!(build_long_response_part(7, OperationId::new(11), 1, 0, &oversized).is_none());
    }
}
