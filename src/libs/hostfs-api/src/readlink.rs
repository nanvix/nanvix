// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Readlink request and response wire format.
//!
//! `readlink` reads the textual target of a symbolic link without following it.
//! The target string is returned verbatim and is not necessarily a path that the
//! daemon (or the guest) can resolve in isolation — the guest is responsible for
//! any further interpretation.
//!
//! # Inline Limit
//!
//! The inline single-message response carries up to [`MAX_INLINE_READLINK_TARGET`]
//! bytes of the link target. Symbolic links whose target exceeds that limit are
//! returned via the multi-part `HostFsReadlinkResponsePart` wire form (see
//! [`long_msg`](crate::long_msg)). The total target length is capped at
//! [`PATH_MAX`](::sysapi::limits::PATH_MAX) bytes; anything longer is reported as
//! [`HOSTFS_ERR_INVALID`](crate::HOSTFS_ERR_INVALID).

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    set_header,
    set_op_id,
    OperationId,
    HOSTFS_DATA_START,
    MAX_INLINE_PATH_LEN,
};
use ::core::mem;
use ::sys::ipc::Message;

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum length, in bytes, of a symbolic link target carried inline in a
/// single-message [`ReadlinkResponse`].
pub const MAX_INLINE_READLINK_TARGET: usize = Message::PAYLOAD_SIZE
    - HOSTFS_DATA_START
    - ReadlinkResponse::SIZE_OF_STATUS
    - ReadlinkResponse::SIZE_OF_TARGET_LEN;

//==================================================================================================
// Structures
//==================================================================================================

/// Readlink request: read the target of a symbolic link.
///
/// Inline single-message form. For paths exceeding [`MAX_INLINE_PATH_LEN`] the caller
/// must use the multi-part variant
/// (`LongReadlinkRequest` in [`long_msg`](crate::long_msg)).
#[derive(Debug, Clone)]
pub struct ReadlinkRequest {
    /// Length of the inline path in bytes.
    pub path_len: u16,
    /// Inline path bytes.
    pub path: [u8; MAX_INLINE_PATH_LEN],
}

/// Readlink response.
///
/// On success, `status` is `0`, `target_len` gives the number of valid bytes in
/// `target`, and bytes beyond `target_len` are undefined. On failure, `status` is a
/// negative `HOSTFS_ERR_*` code and the remaining fields are undefined.
#[derive(Debug, Clone)]
pub struct ReadlinkResponse {
    /// Status: `0` on success, negative `HOSTFS_ERR_*` on failure.
    pub status: i32,
    /// Number of valid bytes in `target`.
    pub target_len: u16,
    /// Inline target bytes.
    pub target: [u8; MAX_INLINE_READLINK_TARGET],
}

impl ReadlinkRequest {
    /// Size of `path_len` field.
    const SIZE_OF_PATH_LEN: usize = mem::size_of::<u16>();
    /// Offset of `path_len` field (relative to data section start).
    const OFFSET_OF_PATH_LEN: usize = 0;
    /// Offset of `path` field (relative to data section start).
    const OFFSET_OF_PATH: usize = Self::OFFSET_OF_PATH_LEN + Self::SIZE_OF_PATH_LEN;

    /// Builds an inline [`ReadlinkRequest`] from a path slice.
    ///
    /// Returns `None` if `path` is longer than [`MAX_INLINE_PATH_LEN`]. Callers must
    /// fall back to the multi-part request form in [`long_msg`](crate::long_msg) when
    /// this returns `None`.
    pub fn from_path(path: &[u8]) -> Option<Self> {
        if path.len() > MAX_INLINE_PATH_LEN {
            return None;
        }
        let mut buf: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
        buf[..path.len()].copy_from_slice(path);
        Some(Self {
            path_len: path.len() as u16,
            path: buf,
        })
    }

    /// Serializes this request into a complete message payload (header + op_id + data).
    pub fn serialize(
        &self,
        header_value: u16,
        op_id: OperationId,
    ) -> [u8; Message::PAYLOAD_SIZE] {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
        set_header(&mut payload, header_value);
        set_op_id(&mut payload, op_id);
        let path_len_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_PATH_LEN;
        let path_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_PATH;
        payload[path_len_off..path_len_off + Self::SIZE_OF_PATH_LEN]
            .copy_from_slice(&self.path_len.to_le_bytes());
        let copy_len: usize = (self.path_len as usize).min(MAX_INLINE_PATH_LEN);
        payload[path_off..path_off + copy_len].copy_from_slice(&self.path[..copy_len]);
        payload
    }

    ///
    /// # Description
    ///
    /// Decodes a `ReadlinkRequest` from the message payload.
    ///
    /// # Returns
    ///
    /// Returns `None` if the encoded `path_len` is larger than the inline path buffer.
    ///
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Option<Self> {
        let path_len_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_PATH_LEN;
        let path_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_PATH;
        let path_len: u16 = u16::from_le_bytes(
            payload[path_len_off..path_len_off + Self::SIZE_OF_PATH_LEN]
                .try_into()
                .ok()?,
        );
        if (path_len as usize) > MAX_INLINE_PATH_LEN {
            return None;
        }
        let mut path: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
        let copy_len: usize = path_len as usize;
        path[..copy_len].copy_from_slice(&payload[path_off..path_off + copy_len]);
        Some(Self { path_len, path })
    }
}

impl ReadlinkResponse {
    /// Size of `status` field.
    const SIZE_OF_STATUS: usize = mem::size_of::<i32>();
    /// Size of `target_len` field.
    const SIZE_OF_TARGET_LEN: usize = mem::size_of::<u16>();
    /// Offset of `status` field (relative to data section start).
    const OFFSET_OF_STATUS: usize = 0;
    /// Offset of `target_len` field (relative to data section start).
    const OFFSET_OF_TARGET_LEN: usize = Self::OFFSET_OF_STATUS + Self::SIZE_OF_STATUS;
    /// Offset of `target` field (relative to data section start).
    const OFFSET_OF_TARGET: usize = Self::OFFSET_OF_TARGET_LEN + Self::SIZE_OF_TARGET_LEN;

    /// Encodes this response into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let status_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_STATUS;
        let target_len_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_TARGET_LEN;
        let target_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_TARGET;
        payload[status_off..status_off + Self::SIZE_OF_STATUS]
            .copy_from_slice(&self.status.to_le_bytes());
        payload[target_len_off..target_len_off + Self::SIZE_OF_TARGET_LEN]
            .copy_from_slice(&self.target_len.to_le_bytes());
        let copy_len: usize = (self.target_len as usize).min(MAX_INLINE_READLINK_TARGET);
        payload[target_off..target_off + copy_len].copy_from_slice(&self.target[..copy_len]);
    }

    ///
    /// # Description
    ///
    /// Decodes a `ReadlinkResponse` from the message payload.
    ///
    /// # Returns
    ///
    /// Returns `None` if the encoded `target_len` exceeds the inline target buffer.
    ///
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Option<Self> {
        let status_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_STATUS;
        let target_len_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_TARGET_LEN;
        let target_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_TARGET;
        let status: i32 = i32::from_le_bytes(
            payload[status_off..status_off + Self::SIZE_OF_STATUS]
                .try_into()
                .ok()?,
        );
        let target_len: u16 = u16::from_le_bytes(
            payload[target_len_off..target_len_off + Self::SIZE_OF_TARGET_LEN]
                .try_into()
                .ok()?,
        );
        if (target_len as usize) > MAX_INLINE_READLINK_TARGET {
            return None;
        }
        let mut target: [u8; MAX_INLINE_READLINK_TARGET] = [0u8; MAX_INLINE_READLINK_TARGET];
        let copy_len: usize = target_len as usize;
        target[..copy_len].copy_from_slice(&payload[target_off..target_off + copy_len]);
        Some(Self {
            status,
            target_len,
            target,
        })
    }
}
