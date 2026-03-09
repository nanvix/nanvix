// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Submission queue entry (SQE) — guest kernel → host.

/// Syscall opcode encoded in each SQE.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SqeOpcode {
    /// No-op (used for benchmarking ring overhead).
    Nop = 0,
    /// write(fd, buf, count).
    Write = 1,
    /// read(fd, buf, count).
    Read = 2,
    /// open(path, flags, mode).
    Open = 3,
    /// close(fd).
    Close = 4,
    /// stat/fstat.
    Stat = 5,
    /// Generic IKC message pass-through (wraps existing Message struct).
    IkcMessage = 254,
    /// Raw bulk data transfer (wraps existing DataChunkHeader).
    BulkData = 255,
}

impl SqeOpcode {
    /// Converts a raw u16 to an opcode, returning `None` for unknown values.
    pub fn from_u16(val: u16) -> Option<Self> {
        match val {
            0 => Some(Self::Nop),
            1 => Some(Self::Write),
            2 => Some(Self::Read),
            3 => Some(Self::Open),
            4 => Some(Self::Close),
            5 => Some(Self::Stat),
            254 => Some(Self::IkcMessage),
            255 => Some(Self::BulkData),
            _ => None,
        }
    }
}

/// Flags for an SQE.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SqeFlags(pub u16);

impl SqeFlags {
    /// No special flags.
    pub const NONE: Self = Self(0);
    /// Buffer address refers to a pre-registered data buffer slot index.
    pub const FIXED_BUF: Self = Self(1 << 0);
    /// This SQE is linked to the next one (process both or neither).
    pub const LINKED: Self = Self(1 << 1);
    /// Data is embedded inline in the SQE (in the `inline_data` field).
    pub const INLINE: Self = Self(1 << 2);
}

/// A single submission queue entry.
///
/// The guest kernel writes one SQE per syscall request. The layout is fixed at 64 bytes
/// to match a cache line and allow indexed access.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SqEntry {
    /// Syscall opcode.
    pub opcode: u16,
    /// Flags (see [`SqeFlags`]).
    pub flags: u16,
    /// File descriptor (for read/write/close/stat).
    pub fd: i32,
    /// Opaque tag returned verbatim in the corresponding CQE.
    pub user_data: u64,
    /// Guest physical address of the data buffer, or data buffer slot index
    /// when `FIXED_BUF` is set, or inline data start when `INLINE` is set.
    pub addr: u64,
    /// Byte count for read/write operations.
    pub len: u32,
    /// File offset for positioned I/O (pread/pwrite).
    pub offset: u32,
    /// Inline data for small payloads (up to 32 bytes). Used when `INLINE` flag is set.
    pub inline_data: [u8; 32],
}

// Compile-time layout assertion.
const _: () = assert!(core::mem::size_of::<SqEntry>() == 64);

impl SqEntry {
    /// Creates a zeroed SQE.
    pub const fn zeroed() -> Self {
        Self {
            opcode: 0,
            flags: 0,
            fd: 0,
            user_data: 0,
            addr: 0,
            len: 0,
            offset: 0,
            inline_data: [0u8; 32],
        }
    }

    /// Creates an SQE for a write operation.
    pub fn new_write(fd: i32, addr: u64, len: u32, user_data: u64) -> Self {
        Self {
            opcode: SqeOpcode::Write as u16,
            flags: SqeFlags::NONE.0,
            fd,
            user_data,
            addr,
            len,
            offset: 0,
            inline_data: [0u8; 32],
        }
    }

    /// Creates an SQE for a read operation.
    pub fn new_read(fd: i32, addr: u64, len: u32, user_data: u64) -> Self {
        Self {
            opcode: SqeOpcode::Read as u16,
            flags: SqeFlags::NONE.0,
            fd,
            user_data,
            addr,
            len,
            offset: 0,
            inline_data: [0u8; 32],
        }
    }

    /// Creates a no-op SQE (for benchmarking).
    pub fn new_nop(user_data: u64) -> Self {
        Self {
            opcode: SqeOpcode::Nop as u16,
            flags: SqeFlags::NONE.0,
            fd: 0,
            user_data,
            addr: 0,
            len: 0,
            offset: 0,
            inline_data: [0u8; 32],
        }
    }

    /// Creates an SQE for a legacy IKC message pass-through.
    pub fn new_ikc_message(addr: u64, len: u32, user_data: u64) -> Self {
        Self {
            opcode: SqeOpcode::IkcMessage as u16,
            flags: SqeFlags::NONE.0,
            fd: 0,
            user_data,
            addr,
            len,
            offset: 0,
            inline_data: [0u8; 32],
        }
    }

    /// Returns `true` if the inline flag is set.
    pub fn is_inline(&self) -> bool {
        (self.flags & SqeFlags::INLINE.0) != 0
    }

    /// Returns `true` if the fixed-buffer flag is set.
    pub fn is_fixed_buf(&self) -> bool {
        (self.flags & SqeFlags::FIXED_BUF.0) != 0
    }
}
