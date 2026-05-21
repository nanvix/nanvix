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

#[cfg(feature = "std")]
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
