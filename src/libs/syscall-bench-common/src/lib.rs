// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Shared protocol definitions for the system call benchmark.
//!
//! This crate is `no_std`-compatible so it can be used by both the guest (`syscall-bench-nostd`)
//! and the host benchmark driver (`nanvix-bench`).

#![no_std]
#![deny(clippy::all)]

//==================================================================================================
// Operation Opcodes
//==================================================================================================

/// Opcode for no-op (protocol round-trip baseline).
const OP_NOOP: u8 = 0x00;
/// Opcode for `getppid()`.
const OP_GETPPID: u8 = 0x01;
/// Opcode for `clock_gettime()`.
const OP_CLOCK_GETTIME: u8 = 0x02;
/// Opcode for `sched_yield()`.
const OP_SCHED_YIELD: u8 = 0x03;
/// Opcode for `getpgrp()`.
const OP_GETPGRP: u8 = 0x04;
/// Opcode for `fstat()`.
const OP_FSTAT: u8 = 0x05;
/// Opcode for `stat()`.
const OP_STAT: u8 = 0x06;
/// Opcode for `read()`.
const OP_READ: u8 = 0x07;
/// Opcode for `write()`.
const OP_WRITE: u8 = 0x08;
/// Opcode for `open()` + `close()`.
const OP_OPEN_CLOSE: u8 = 0x09;

/// Operations exchanged between the host driver and the guest benchmark program.
///
/// The host asks the guest to issue an operation a given number of times back to back; the guest
/// replies with a one-byte acknowledgement once the whole batch completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SyscallOp {
    /// No-op: immediate acknowledgement, used to measure the protocol round-trip overhead.
    Noop = OP_NOOP,
    /// `getppid()`, served by the kernel alone (null kernel call).
    GetPpid = OP_GETPPID,
    /// `clock_gettime(CLOCK_MONOTONIC)`, served by the kernel with a copy out to user space.
    ClockGettime = OP_CLOCK_GETTIME,
    /// `sched_yield()`, served by the kernel scheduler.
    SchedYield = OP_SCHED_YIELD,
    /// `getpgrp()`, served by the process manager daemon.
    GetPgrp = OP_GETPGRP,
    /// `fstat()` on an open `/dev/null` descriptor, served by the VFS daemon.
    Fstat = OP_FSTAT,
    /// `stat()` on `/dev/null`, served by the VFS daemon.
    Stat = OP_STAT,
    /// One-byte `read()` from an open `/dev/null` descriptor, served by the VFS daemon.
    Read = OP_READ,
    /// One-byte `write()` to an open `/dev/null` descriptor, served by the VFS daemon.
    Write = OP_WRITE,
    /// `open()` + `close()` of `/dev/null`, served by the VFS daemon.
    OpenClose = OP_OPEN_CLOSE,
}

impl SyscallOp {
    /// Converts a raw byte to a [`SyscallOp`], returning `None` for unknown opcodes.
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            OP_NOOP => Some(Self::Noop),
            OP_GETPPID => Some(Self::GetPpid),
            OP_CLOCK_GETTIME => Some(Self::ClockGettime),
            OP_SCHED_YIELD => Some(Self::SchedYield),
            OP_GETPGRP => Some(Self::GetPgrp),
            OP_FSTAT => Some(Self::Fstat),
            OP_STAT => Some(Self::Stat),
            OP_READ => Some(Self::Read),
            OP_WRITE => Some(Self::Write),
            OP_OPEN_CLOSE => Some(Self::OpenClose),
            _ => None,
        }
    }

    /// Returns the opcode as a raw `u8`.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

//==================================================================================================
// Request Frames
//==================================================================================================

/// Size (in bytes) of a request frame.
///
/// The wire format of a request is `[opcode: u8][count: u32 little-endian]`, where `count` is the
/// number of times that the guest issues the operation back to back. The guest ignores `count` for
/// [`SyscallOp::Noop`].
pub const REQUEST_SIZE: usize = 1 + ::core::mem::size_of::<u32>();

/// Encodes a request for `count` back-to-back executions of `op`.
pub const fn encode_request(op: SyscallOp, count: u32) -> [u8; REQUEST_SIZE] {
    let count_bytes: [u8; 4] = count.to_le_bytes();
    [
        op.as_u8(),
        count_bytes[0],
        count_bytes[1],
        count_bytes[2],
        count_bytes[3],
    ]
}

/// Decodes a request frame into its operation and batch size.
///
/// Returns `None` when the frame carries an unknown opcode.
pub const fn decode_request(frame: &[u8; REQUEST_SIZE]) -> Option<(SyscallOp, u32)> {
    match SyscallOp::from_u8(frame[0]) {
        Some(op) => Some((op, u32::from_le_bytes([frame[1], frame[2], frame[3], frame[4]]))),
        None => None,
    }
}

//==================================================================================================
// Acknowledgement Bytes
//==================================================================================================

/// Acknowledgement byte: success.
///
/// The guest also sends this byte once, before serving any request, to report that it is ready.
pub const ACK_OK: u8 = 0x00;

/// Acknowledgement byte: error.
///
/// The guest also sends this byte instead of [`ACK_OK`] when it fails to get ready.
pub const ACK_ERR: u8 = 0xFF;
