// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # nvx-ring — Lock-Free SPSC Ring Buffer for Guest–Host IPC
//!
//! This crate provides the shared data structures for a single-producer, single-consumer (SPSC)
//! ring buffer used to batch syscall requests between a guest kernel and the host VMM/linuxd.
//!
//! The ring lives in a shared memory region at a fixed guest physical address and is accessed by
//! both the guest kernel (producer of SQEs, consumer of CQEs) and the host (consumer of SQEs,
//! producer of CQEs).
//!
//! All types are `#[repr(C)]` with stable layout so that guest (no_std) and host (std) agree on
//! the byte representation.

#![no_std]
#![deny(clippy::unwrap_used, clippy::expect_used)]

mod control;
mod cqe;
mod ring;
mod sqe;

pub use control::{
    CqFlags,
    RingControl,
    SqFlags,
};
pub use cqe::{
    CqEntry,
    CqeFlags,
};
pub use ring::{
    RingConsumer,
    RingProducer,
};
pub use sqe::{
    SqEntry,
    SqeFlags,
    SqeOpcode,
};

/// Number of entries in the submission queue (must be a power of two).
pub const SQ_SIZE: u32 = 256;

/// Number of entries in the completion queue (must be a power of two).
pub const CQ_SIZE: u32 = 256;

/// Byte offset of the control block within the shared region.
pub const CONTROL_OFFSET: usize = 0;

/// Byte offset of the submission queue within the shared region.
pub const SQ_OFFSET: usize = 256;

/// Byte offset of the completion queue within the shared region.
pub const CQ_OFFSET: usize = SQ_OFFSET + (SQ_SIZE as usize) * core::mem::size_of::<SqEntry>();

/// Byte offset of the message-data region.
pub const DATA_OFFSET: usize = CQ_OFFSET + (CQ_SIZE as usize) * core::mem::size_of::<CqEntry>();

/// Total size of the shared ring buffer region in bytes.
pub const REGION_SIZE: usize = 2 * 1024 * 1024; // 2 MiB.

/// Size of each message-data slot in bytes (fits one serialized IPC message).
pub const DATA_SLOT_SIZE: usize = 512;

/// Number of message-data slots available in the message-data region.
pub const DATA_SLOT_COUNT: usize = CQ_SIZE as usize;

/// Size in bytes reserved for message-data slots.
pub const DATA_SLOT_REGION_SIZE: usize = DATA_SLOT_COUNT * DATA_SLOT_SIZE;

/// Byte offset of the fixed-buffer region used for large payload transfers.
pub const FIXED_BUF_OFFSET: usize = DATA_OFFSET + DATA_SLOT_REGION_SIZE;

/// Size of each fixed buffer in bytes.
pub const FIXED_BUF_SIZE: usize = 4096;

/// Number of fixed buffers available in the shared region.
pub const FIXED_BUF_COUNT: usize = (REGION_SIZE - FIXED_BUF_OFFSET) / FIXED_BUF_SIZE;

// -- Control block field offsets (must match RingControl repr(C) layout) --

/// Byte offset of `sq_head` within the control block.
pub const CTRL_SQ_HEAD: usize = 0;
/// Byte offset of `sq_tail` within the control block.
pub const CTRL_SQ_TAIL: usize = 4;
/// Byte offset of `sq_mask` within the control block.
pub const CTRL_SQ_MASK: usize = 8;
/// Byte offset of `sq_flags` within the control block.
pub const CTRL_SQ_FLAGS: usize = 12;
/// Byte offset of `cq_head` within the control block.
pub const CTRL_CQ_HEAD: usize = 64;
/// Byte offset of `cq_tail` within the control block.
pub const CTRL_CQ_TAIL: usize = 68;
/// Byte offset of `cq_mask` within the control block.
pub const CTRL_CQ_MASK: usize = 72;
/// Byte offset of `cq_flags` within the control block.
pub const CTRL_CQ_FLAGS: usize = 76;

/// Byte offset of the host-side SQ notification sequence word.
///
/// This lives in the padding between the control block and the SQ so the shared
/// region layout for SQ/CQ/data does not change.
pub const HOST_SQ_SIGNAL_OFFSET: usize = 128;

/// Byte offset of the host-side CQ notification sequence word.
pub const HOST_CQ_SIGNAL_OFFSET: usize = 132;
