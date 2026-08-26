// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Wire format for the host filesystem daemon (`hostfsd`).
//!
//! This crate defines the binary protocol used for communication between the guest-side VFS daemon
//! (`vfsd`) and the host-side filesystem daemon (`hostfsd`). Messages are encoded into the
//! fixed-size IPC message payload (`Message::PAYLOAD_SIZE` bytes) using a compact binary format.
//!
//! # Protocol Overview
//!
//! All messages use the [`SystemCallMessage`] wire format. Its six-byte
//! [`SystemCallMessageKind`] contains a two-byte [`SystemCallMessageKind`] discriminant and a
//! four-byte operation identifier (`op_id`); the remaining bytes carry operation-specific data.
//! The `op_id` is assigned by vfsd when sending a request and echoed back by hostfsd in the
//! corresponding response, allowing vfsd to match responses to pending operations without relying
//! on FIFO ordering.
//!
//! Each host filesystem operation maps to a pair of message kinds (request and response)
//! defined in the `syscall` crate.

#![cfg_attr(not(feature = "std"), no_std)]

// The wire format uses native-endian encoding for the kind and op_id fields
// (matching the `SystemCallMessage` packed struct layout) and explicit little-endian
// for operation-specific data. This is only correct on little-endian hosts.
#[cfg(not(target_endian = "little"))]
compile_error!("hostfs-api wire format requires a little-endian target");

use ::sys::ipc::Message;

//==================================================================================================
// Modules
//==================================================================================================

mod chown;
mod close;
mod error;
mod flush;
pub mod long_msg;
mod lseek;
mod lstat;
mod mkdir;
mod open;
mod read;
mod readdir;
mod readlink;
mod rename;
mod rmdir;
mod stat;
mod stat_times;
mod truncate;
mod unlink;
mod write;

pub use self::{
    chown::ChownRequest,
    close::CloseRequest,
    flush::FlushRequest,
    lseek::{
        LseekRequest,
        LseekResponse,
    },
    lstat::{
        file_kind,
        LstatRequest,
        LstatResponse,
    },
    mkdir::MkdirRequest,
    open::{
        OpenRequest,
        OpenResponse,
    },
    read::{
        ReadRequest,
        ReadResponse,
    },
    readdir::{
        ReadDirEntry,
        ReadDirRequest,
    },
    readlink::{
        ReadlinkRequest,
        ReadlinkResponse,
        MAX_INLINE_READLINK_TARGET,
    },
    rename::RenameRequest,
    rmdir::RmdirRequest,
    stat::{
        StatRequest,
        StatResponse,
    },
    stat_times::{
        StatTime,
        StatTimesResponse,
    },
    truncate::TruncateRequest,
    unlink::UnlinkRequest,
    write::{
        WriteRequest,
        WriteResponse,
    },
};

//==================================================================================================
// Operation Identifier
//==================================================================================================

/// Strongly-typed wrapper around a `u32` operation identifier.
///
/// Each outgoing IKC request carries an `OperationId` assigned by vfsd. hostfsd echoes
/// it back in the response so vfsd can match responses to pending operations.
///
/// The wire format stores the identifier as a 4-byte little-endian `u32` at payload
/// bytes `[2..6]`, immediately after the 2-byte header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(u32);

impl OperationId {
    /// Sentinel value used in error responses when the real operation identifier
    /// cannot be recovered (e.g., the assembled buffer is too short to contain one).
    ///
    /// The allocator never issues `u32::MAX`, so this value is guaranteed not to
    /// collide with any live operation.
    pub const INVALID: Self = Self(u32::MAX);

    /// Sentinel value used for fire-and-forget requests that expect no completion,
    /// such as best-effort closes issued when the originating caller is gone (e.g. on
    /// process exit, or when an open completes but the local descriptor cannot be
    /// allocated). No pending op is registered for these, and vfsd's main loop
    /// recognizes responses carrying this id and discards them without logging.
    ///
    /// [`OperationIdAllocator::alloc`] explicitly skips `0`, so this value is guaranteed not to
    /// collide with any live operation — including after the allocator's counter wraps around.
    pub const FIRE_AND_FORGET: Self = Self(0);

    /// Size in bytes of the little-endian wire representation of an `OperationId`.
    ///
    /// Matches the length of the array produced by [`to_le_bytes`](Self::to_le_bytes)
    /// and consumed by [`from_le_bytes`](Self::from_le_bytes).
    pub const SERIALIZED_SIZE: usize = core::mem::size_of::<u32>();

    /// Creates a new operation identifier from a raw `u32` value.
    ///
    /// This is crate-internal: only [`get_op_id`] and [`OperationIdAllocator`] should
    /// construct identifiers.
    pub(crate) const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw `u32` value.
    ///
    /// This is crate-internal: only [`set_op_id`] needs the raw representation for
    /// wire serialization.
    pub(crate) const fn raw(self) -> u32 {
        self.0
    }

    /// Returns the little-endian byte representation of the operation identifier.
    ///
    /// Used by vfsd to serialize the identifier into multi-part request buffers.
    pub const fn to_le_bytes(self) -> [u8; 4] {
        self.0.to_le_bytes()
    }

    /// Reconstructs an operation identifier from its little-endian byte representation.
    ///
    /// Used by hostfsd to extract the identifier from assembled multi-part request bytes.
    pub const fn from_le_bytes(bytes: [u8; 4]) -> Self {
        Self(u32::from_le_bytes(bytes))
    }
}

#[cfg(any(test, feature = "test-support"))]
impl OperationId {
    /// Creates an operation identifier from a raw value.
    ///
    /// This is intended **only** for tests that need to forge specific identifiers.
    /// Production code must obtain identifiers through [`OperationIdAllocator::alloc`]
    /// or by deserializing a wire message with [`get_op_id`].
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }
}

impl core::fmt::Display for OperationId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

//==================================================================================================
// Operation Identifier Allocator
//==================================================================================================

/// Sequential allocator for [`OperationId`] values.
///
/// This is the only public way to obtain a fresh `OperationId` in production code.
/// The allocator maintains a monotonic counter and skips identifiers that are still
/// in use (as reported by a caller-supplied predicate), handling `u32` wrap-around.
pub struct OperationIdAllocator {
    next_id: u32,
}

impl Default for OperationIdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationIdAllocator {
    /// Creates a new allocator starting at identifier 1.
    ///
    /// Identifier `0` is a valid wire value (the set/get helpers accept it) but is reserved as the
    /// [`OperationId::FIRE_AND_FORGET`] sentinel, so [`alloc`](Self::alloc) never issues it.
    pub const fn new() -> Self {
        Self { next_id: 1 }
    }

    /// Allocates the next unique operation identifier.
    ///
    /// The `in_use` predicate is called to skip identifiers that are currently active
    /// (e.g., present in a pending-operation map). This guarantees the returned ID does
    /// not collide with any live operation. The allocator also skips the reserved sentinels
    /// [`OperationId::INVALID`] (`u32::MAX`, used for error responses) and
    /// [`OperationId::FIRE_AND_FORGET`] (`0`, used for completion-less requests), so a returned
    /// identifier never collides with either reserved value — including after wrap-around.
    pub fn alloc(&mut self, in_use: impl Fn(&OperationId) -> bool) -> OperationId {
        let mut id: u32 = self.next_id;
        while id == OperationId::INVALID.raw()
            || id == OperationId::FIRE_AND_FORGET.raw()
            || in_use(&OperationId(id))
        {
            id = id.wrapping_add(1);
        }
        self.next_id = id.wrapping_add(1);
        OperationId(id)
    }
}

//==================================================================================================
// Constants
//==================================================================================================

/// Byte offset where operation-specific data begins in the message payload.
///
/// Layout: `[header: u16][op_id: u32][data...]`
pub const HOSTFS_DATA_START: usize = 6;

/// Maximum path length that fits inline in a single message payload.
///
/// Derived from the live [`Message::PAYLOAD_SIZE`] so it tracks the IPC payload size automatically
/// (the kernel-stamped client identity in nanvix/nanvix#2662 shrank the payload). The binding
/// (tightest) inline path request is `open`/`mkdir`, whose data section is
/// `[flags|mode: u32][path_len: u16][path...]`, i.e. 6 bytes precede the path. Longer paths fall
/// back to the multi-part form in [`long_msg`].
pub const MAX_INLINE_PATH_LEN: usize = Message::PAYLOAD_SIZE - HOSTFS_DATA_START - 4 - 2;

/// Maximum inline data length for read responses.
///
/// The read-response data section is `[bytes_read: i32][data...]`, so 4 bytes precede the data.
pub const MAX_INLINE_READ_DATA: usize = Message::PAYLOAD_SIZE - HOSTFS_DATA_START - 4;

/// Maximum inline data length for write requests.
///
/// The write-request data section is `[fd: i32][count: u32][offset: i64][data_len: u16][data...]`,
/// so 18 bytes precede the data. The guest-side VFS layer (`handle_write_with_hostfs`) clamps the
/// write buffer to this size before encoding the request, so `count` and `data_len` never exceed it
/// on the wire. The guest issues multiple write requests to transfer larger buffers, observing the
/// returned `bytes_written` count and retrying the remainder.
pub const MAX_INLINE_WRITE_DATA: usize =
    Message::PAYLOAD_SIZE - HOSTFS_DATA_START - (4 + 4 + 8 + 2);

/// Maximum filename length in a directory entry response.
///
/// The directory-entry data section is `[name_len: u16][is_dir: u8][size: u64][name...]`, so 11
/// bytes precede the name. Names longer than this are truncated to the inline capacity.
pub const MAX_DIR_ENTRY_NAME_LEN: usize = Message::PAYLOAD_SIZE - HOSTFS_DATA_START - (2 + 1 + 8);

//==================================================================================================
// Error Codes
//==================================================================================================

pub use self::error::{
    HOSTFS_ERR_EXISTS,
    HOSTFS_ERR_INVALID,
    HOSTFS_ERR_IO,
    HOSTFS_ERR_IS_DIR,
    HOSTFS_ERR_LOOP,
    HOSTFS_ERR_NOT_DIR,
    HOSTFS_ERR_NOT_EMPTY,
    HOSTFS_ERR_NOT_FOUND,
    HOSTFS_ERR_NOT_PERMITTED,
    HOSTFS_ERR_NOT_SUPPORTED,
    HOSTFS_ERR_PERMISSION,
};

//==================================================================================================
// Serialization
//==================================================================================================

/// Sets the [`SystemCallMessageKind`] discriminant in the first two bytes of a message payload.
///
/// This writes the raw `u16` value of the given message kind into `payload[0..2]` using
/// native-endian byte order, matching the `#[repr(C, packed)]` layout of `SystemCallMessage`.
///
/// NOTE: kind and op_id fields use native-endian encoding (matching the `SystemCallMessage`
/// struct layout), while operation-specific data fields use explicit little-endian encoding.
/// This is safe because all supported targets are little-endian (enforced by the compile-time
/// assertion below).
pub fn set_kind(payload: &mut [u8; Message::PAYLOAD_SIZE], kind_value: u16) {
    payload[0..2].copy_from_slice(&kind_value.to_ne_bytes());
}

/// Writes operation-specific data into the payload after the composite message header.
///
/// This is a convenience helper for building simple responses where the data
/// portion is a small byte slice (e.g., a status code).
pub fn set_payload_data(payload: &mut [u8; Message::PAYLOAD_SIZE], data: &[u8]) {
    let copy_len: usize = data.len().min(Message::PAYLOAD_SIZE - HOSTFS_DATA_START);
    payload[HOSTFS_DATA_START..HOSTFS_DATA_START + copy_len].copy_from_slice(&data[..copy_len]);
}

/// Writes the operation identifier into the payload at bytes 2..6.
///
/// The op_id is assigned by vfsd when sending a request and must be echoed back
/// by hostfsd in the corresponding response.
pub fn set_op_id(payload: &mut [u8; Message::PAYLOAD_SIZE], op_id: OperationId) {
    payload[2..6].copy_from_slice(&op_id.raw().to_ne_bytes());
}

/// Reads the operation identifier from the payload at bytes 2..6.
pub fn get_op_id(payload: &[u8; Message::PAYLOAD_SIZE]) -> OperationId {
    OperationId::new(u32::from_ne_bytes([payload[2], payload[3], payload[4], payload[5]]))
}
