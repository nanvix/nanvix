// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Lstat request and response wire format.
//!
//! `lstat` is the POSIX variant of `stat` that does *not* follow the final symbolic
//! link component of the path. The request takes a path (rather than an FD) because
//! a symbolic link cannot be opened without following it (POSIX `open` follows links
//! by default and `O_NOFOLLOW` fails with `ELOOP` on a link). The response mirrors
//! [`StatResponse`](crate::StatResponse) with an extra `kind` field that distinguishes
//! regular files, directories, and symbolic links.

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
// Structures
//==================================================================================================

/// File kind reported by an [`LstatResponse`].
///
/// These values are deliberately small integers (not POSIX `S_IF*` mode bits) so the
/// wire format does not depend on host or guest libc mode-bit definitions. The guest
/// is responsible for translating this value into whatever its VFS layer expects.
pub mod file_kind {
    /// Regular file.
    pub const REGULAR: u8 = 0;
    /// Directory.
    pub const DIRECTORY: u8 = 1;
    /// Symbolic link.
    pub const SYMLINK: u8 = 2;
    /// Anything else (block/char device, FIFO, socket, unknown).
    pub const OTHER: u8 = 3;
}

/// Lstat request: stat a path without following the final symbolic link.
///
/// Inline single-message form. For paths that exceed [`MAX_INLINE_PATH_LEN`], the
/// caller must use the multi-part variant
/// (`LongLstatRequest` in [`long_msg`](crate::long_msg)).
#[derive(Debug, Clone)]
pub struct LstatRequest {
    /// Length of the inline path in bytes.
    pub path_len: u16,
    /// Inline path bytes.
    pub path: [u8; MAX_INLINE_PATH_LEN],
}

/// Lstat response.
///
/// The `status` field is `0` on success or a negative `HOSTFS_ERR_*` code on failure.
/// On error, the remaining fields are undefined.
#[derive(Debug, Clone, Copy)]
pub struct LstatResponse {
    /// Status: `0` on success, negative `HOSTFS_ERR_*` on failure.
    pub status: i32,
    /// File size in bytes (link length in bytes for symbolic links).
    pub size: u64,
    /// Host file mode bits.
    ///
    /// On Unix hosts this is the raw `st_mode` (permission bits plus type bits). On
    /// Windows hosts this is a synthetic value: the type bits are best-effort and
    /// the permission bits are derived from the read-only attribute, so only the
    /// permission bits should be considered meaningful by the guest. In all cases
    /// the authoritative file-type discriminant is [`Self::kind`], not the type
    /// bits embedded in `mode`.
    pub mode: u32,
    /// File kind discriminant; see [`file_kind`].
    pub kind: u8,
}

impl LstatRequest {
    /// Size of `path_len` field.
    const SIZE_OF_PATH_LEN: usize = mem::size_of::<u16>();
    /// Offset of `path_len` field (relative to data section start).
    const OFFSET_OF_PATH_LEN: usize = 0;
    /// Offset of `path` field (relative to data section start).
    const OFFSET_OF_PATH: usize = Self::OFFSET_OF_PATH_LEN + Self::SIZE_OF_PATH_LEN;

    /// Builds an inline [`LstatRequest`] from a path slice.
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
    /// Decodes a `LstatRequest` from the message payload.
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

impl LstatResponse {
    /// Size of `status` field.
    const SIZE_OF_STATUS: usize = mem::size_of::<i32>();
    /// Size of `size` field.
    const SIZE_OF_SIZE: usize = mem::size_of::<u64>();
    /// Size of `mode` field.
    const SIZE_OF_MODE: usize = mem::size_of::<u32>();
    /// Size of `kind` field.
    const SIZE_OF_KIND: usize = mem::size_of::<u8>();
    /// Offset of `status` field (relative to data section start).
    const OFFSET_OF_STATUS: usize = 0;
    /// Offset of `size` field (relative to data section start).
    const OFFSET_OF_SIZE: usize = Self::OFFSET_OF_STATUS + Self::SIZE_OF_STATUS;
    /// Offset of `mode` field (relative to data section start).
    const OFFSET_OF_MODE: usize = Self::OFFSET_OF_SIZE + Self::SIZE_OF_SIZE;
    /// Offset of `kind` field (relative to data section start).
    const OFFSET_OF_KIND: usize = Self::OFFSET_OF_MODE + Self::SIZE_OF_MODE;

    /// Encodes this response into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let status_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_STATUS;
        let size_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_SIZE;
        let mode_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_MODE;
        let kind_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_KIND;
        payload[status_off..status_off + Self::SIZE_OF_STATUS]
            .copy_from_slice(&self.status.to_le_bytes());
        payload[size_off..size_off + Self::SIZE_OF_SIZE].copy_from_slice(&self.size.to_le_bytes());
        payload[mode_off..mode_off + Self::SIZE_OF_MODE].copy_from_slice(&self.mode.to_le_bytes());
        payload[kind_off] = self.kind;
    }

    ///
    /// # Description
    ///
    /// Decodes an `LstatResponse` from the message payload.
    ///
    /// # Returns
    ///
    /// Returns `None` if the encoded `target_len` exceeds the inline target buffer.
    ///
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Option<Self> {
        let status_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_STATUS;
        let size_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_SIZE;
        let mode_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_MODE;
        let kind_off: usize = HOSTFS_DATA_START + Self::OFFSET_OF_KIND;
        let status: i32 = i32::from_le_bytes(
            payload[status_off..status_off + Self::SIZE_OF_STATUS]
                .try_into()
                .ok()?,
        );
        let size: u64 = u64::from_le_bytes(
            payload[size_off..size_off + Self::SIZE_OF_SIZE]
                .try_into()
                .ok()?,
        );
        let mode: u32 = u32::from_le_bytes(
            payload[mode_off..mode_off + Self::SIZE_OF_MODE]
                .try_into()
                .ok()?,
        );
        let kind: u8 = u8::from_le_bytes(
            payload[kind_off..kind_off + Self::SIZE_OF_KIND]
                .try_into()
                .ok()?,
        );
        Some(Self {
            status,
            size,
            mode,
            kind,
        })
    }
}
