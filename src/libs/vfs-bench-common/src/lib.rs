// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Shared protocol definitions for the VFS benchmark.
//!
//! This crate is `no_std`-compatible so it can be used by both the guest (`vfs-bench-nostd`) and
//! the host benchmark driver (`nanvix-bench`).

#![no_std]
#![deny(clippy::all)]

//==================================================================================================
// VFS Operation Opcodes
//==================================================================================================

// Opcode byte values — ordered to match `DECOMPOSED_OPS` dependency levels.

/// Opcode for no-op (protocol round-trip baseline).
const OP_NOOP: u8 = 0x00;
/// Opcode for `stat()`.
const OP_STAT: u8 = 0x01;
/// Opcode for `open()` + `close()`.
const OP_OPEN_CLOSE: u8 = 0x02;
/// Opcode for `read_dir()`.
const OP_READDIR: u8 = 0x07;
/// Opcode for scratch directory creation + teardown.
const OP_CREATE_SCRATCH: u8 = 0x0A;
/// Opcode for sequential read.
const OP_SEQ_READ: u8 = 0x03;
/// Opcode for file creation + deletion.
const OP_CREATE_UNLINK: u8 = 0x06;
/// Opcode for `mkdir()` + `rmdir()`.
const OP_MKDIR_RMDIR: u8 = 0x05;
/// Opcode for `rename()`.
const OP_RENAME: u8 = 0x09;
/// Opcode for sequential write.
const OP_SEQ_WRITE: u8 = 0x04;

/// Operation opcodes exchanged between the host driver and the guest benchmark program.
///
/// The host sends a one-byte opcode on stdin; the guest executes the corresponding VFS operation
/// and replies with a one-byte acknowledgement on stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VfsOp {
    /// No-op: immediate ACK, used to measure protocol round-trip overhead.
    Noop = OP_NOOP,
    /// `stat()` on an existing file.
    Stat = OP_STAT,
    /// `open()` + `close()` on an existing file.
    OpenClose = OP_OPEN_CLOSE,
    /// `read_dir()` on a directory.
    Readdir = OP_READDIR,
    /// `create_mount()` + `unmount()` cycle for a scratch FAT directory.
    CreateScratch = OP_CREATE_SCRATCH,
    /// Sequential read of a file.
    SeqRead = OP_SEQ_READ,
    /// File creation + deletion cycle.
    CreateUnlink = OP_CREATE_UNLINK,
    /// `mkdir()` + `rmdir()` cycle.
    MkdirRmdir = OP_MKDIR_RMDIR,
    /// `rename()` on a file.
    Rename = OP_RENAME,
    /// Sequential write to a scratch file.
    SeqWrite = OP_SEQ_WRITE,
}

impl VfsOp {
    /// Converts a raw byte to a [`VfsOp`], returning `None` for unknown opcodes.
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            OP_NOOP => Some(Self::Noop),
            OP_STAT => Some(Self::Stat),
            OP_OPEN_CLOSE => Some(Self::OpenClose),
            OP_READDIR => Some(Self::Readdir),
            OP_CREATE_SCRATCH => Some(Self::CreateScratch),
            OP_SEQ_READ => Some(Self::SeqRead),
            OP_CREATE_UNLINK => Some(Self::CreateUnlink),
            OP_MKDIR_RMDIR => Some(Self::MkdirRmdir),
            OP_RENAME => Some(Self::Rename),
            OP_SEQ_WRITE => Some(Self::SeqWrite),
            _ => None,
        }
    }

    /// Returns the opcode as a raw `u8`.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

//==================================================================================================
// Acknowledgement Bytes
//==================================================================================================

/// Acknowledgement byte: success.
pub const ACK_OK: u8 = 0x00;

/// Acknowledgement byte: error.
pub const ACK_ERR: u8 = 0xFF;

//==================================================================================================
// Protocol Constants
//==================================================================================================

/// Maximum path length (in bytes) that can be sent in the benchmark protocol.
///
/// The wire format for each request is `[opcode: u8][path_len: u8][path: path_len bytes]`.
/// Operations that do not need a path send `path_len = 0`.
pub const MAX_PATH_LEN: usize = 255;

//==================================================================================================
// Mount Configuration
//==================================================================================================

/// Mount configuration byte: mount the ramfs as writable.
///
/// Uses a dedicated magic value so mount configuration bytes cannot be confused with operation
/// opcodes or acknowledgment bytes during protocol mismatches.
pub const MOUNT_WRITABLE: u8 = 0xA5;

/// Mount configuration byte: mount the ramfs as read-only.
///
/// Enforces the read-only gate on write operations.
/// Uses a distinct magic value to avoid overlapping other one-byte protocol fields.
pub const MOUNT_READONLY: u8 = 0x5A;

//==================================================================================================
// Image Filename
//==================================================================================================

/// Filename of the VFS benchmark FAT image, read from the `VFS_BENCH_IMG` environment variable
/// at compile time. Falls back to `"vfs-bench.img"` when the variable is not set (e.g.,
/// rust-analyzer).
pub const VFS_BENCH_IMG: &str = match option_env!("VFS_BENCH_IMG") {
    Some(v) => v,
    None => "vfs-bench.img",
};
