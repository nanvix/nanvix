// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Completion queue entry (CQE) — host → guest kernel.

/// Flags for a CQE.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CqeFlags(pub u32);

impl CqeFlags {
    /// No special flags.
    pub const NONE: Self = Self(0);
    /// More CQEs are being posted in this batch (coalesce interrupt).
    pub const MORE: Self = Self(1 << 0);
    /// The `buffer_id` field contains a valid pre-registered buffer index.
    pub const BUFFER: Self = Self(1 << 1);
    /// This CQE completes the entire logical fixed-buffer transfer in one shot.
    pub const BATCH: Self = Self(1 << 2);
}

/// A single completion queue entry.
///
/// The host writes one CQE per completed syscall. The guest reads CQEs to collect results.
/// Layout is 64 bytes to match SQE size and cache line.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CqEntry {
    /// Opaque tag copied from the corresponding SQE's `user_data`.
    pub user_data: u64,
    /// Syscall return value (positive = success count, negative = -errno).
    pub result: i64,
    /// Flags (see [`CqeFlags`]).
    pub flags: u32,
    /// Pre-registered buffer index (valid when `CqeFlags::BUFFER` is set).
    pub buffer_id: u32,
    /// Reserved for future use.
    _reserved: [u8; 40],
}

// Compile-time layout assertion.
const _: () = assert!(core::mem::size_of::<CqEntry>() == 64);

impl CqEntry {
    /// Creates a zeroed CQE.
    pub const fn zeroed() -> Self {
        Self {
            user_data: 0,
            result: 0,
            flags: CqeFlags::NONE.0,
            buffer_id: 0,
            _reserved: [0u8; 40],
        }
    }

    /// Creates a CQE with the given result.
    pub fn new(user_data: u64, result: i64) -> Self {
        Self {
            user_data,
            result,
            flags: CqeFlags::NONE.0,
            buffer_id: 0,
            _reserved: [0u8; 40],
        }
    }

    /// Creates a CQE with the MORE flag set (batching hint).
    pub fn new_with_more(user_data: u64, result: i64) -> Self {
        Self {
            user_data,
            result,
            flags: CqeFlags::MORE.0,
            buffer_id: 0,
            _reserved: [0u8; 40],
        }
    }
}
