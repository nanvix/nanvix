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
//! - **Symlink**: `[op_id:4][target_len:2][linkpath_len:2][target:N][linkpath:M]`
//! - **Readlink**: `[op_id:4][path_len:2][path:N]`
//! - **Lstat**: `[op_id:4][path_len:2][path:N]`
//!
//! # Long Response Format
//!
//! - **Readlink response**: `[op_id:4][status:4][target_len:2][target:N]`. Used by
//!   `HostFsReadlinkResponsePart` messages when the target string exceeds the inline
//!   response capacity. Errors are always reported via the single-message
//!   `HostFsReadlinkResponse` form; the multi-part response is only emitted for a
//!   successful `readlink` whose target does not fit inline.

extern crate alloc;

use alloc::vec::Vec;

use crate::OperationId;

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum length of a single path in the long-message wire format (u16::MAX).
pub const MAX_PATH_LEN: usize = u16::MAX as usize;

/// Header size for long open: op_id(4) + flags(4) + path_len(2) = 10
pub const OPEN_HEADER_SIZE: usize = 10;

/// Header size for long unlink: op_id(4) + path_len(2) = 6
pub const UNLINK_HEADER_SIZE: usize = 6;

/// Header size for long rmdir: op_id(4) + path_len(2) = 6
pub const RMDIR_HEADER_SIZE: usize = 6;

/// Header size for long mkdir: op_id(4) + mode(4) + path_len(2) = 10
pub const MKDIR_HEADER_SIZE: usize = 10;

/// Header size for long rename: op_id(4) + old_path_len(2) + new_path_len(2) = 8
pub const RENAME_HEADER_SIZE: usize = 8;

/// Header size for long symlink: op_id(4) + target_len(2) + linkpath_len(2) = 8
pub const SYMLINK_HEADER_SIZE: usize = 8;

/// Header size for long readlink: op_id(4) + path_len(2) = 6
pub const READLINK_HEADER_SIZE: usize = 6;

/// Header size for long lstat: op_id(4) + path_len(2) = 6
pub const LSTAT_HEADER_SIZE: usize = 6;

/// Header size for the long Readlink *response* body:
/// `op_id(4) + status(4) + target_len(2) = 10`.
pub const READLINK_RESPONSE_HEADER_SIZE: usize = 10;

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

//==================================================================================================
// Deserialization (hostfsd, std)
//==================================================================================================

/// Result of deserializing a long OPEN request.
#[cfg(feature = "std")]
pub struct LongOpenRequest {
    pub op_id: OperationId,
    pub flags: i32,
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
    let path_len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    if bytes.len() < OPEN_HEADER_SIZE + path_len {
        return None;
    }
    let path = std::string::String::from_utf8(
        bytes[OPEN_HEADER_SIZE..OPEN_HEADER_SIZE + path_len].to_vec(),
    )
    .ok()?;
    Some(LongOpenRequest { op_id, flags, path })
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

//==================================================================================================
// Serialization (vfsd, no_std + alloc)
//==================================================================================================

/// Serializes the body of a long OPEN request.
///
/// Wire format: `[op_id:4][flags:4][path_len:2][path:N]`. Returns `None` if `path`
/// is longer than [`MAX_PATH_LEN`].
pub fn serialize_long_open_request(
    op_id: OperationId,
    flags: i32,
    path: &[u8],
) -> Option<Vec<u8>> {
    let path_len: u16 = u16::try_from(path.len()).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(OPEN_HEADER_SIZE + path.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
    buf.extend_from_slice(&flags.to_le_bytes());
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
pub fn serialize_long_mkdir_request(
    op_id: OperationId,
    mode: u32,
    path: &[u8],
) -> Option<Vec<u8>> {
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
    let mut buf: Vec<u8> =
        Vec::with_capacity(RENAME_HEADER_SIZE + old_path.len() + new_path.len());
    buf.extend_from_slice(&op_id.to_le_bytes());
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
    let mut buf: Vec<u8> =
        Vec::with_capacity(SYMLINK_HEADER_SIZE + target.len() + linkpath.len());
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
