// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Ring buffer control block shared between guest and host.

use core::sync::atomic::{
    AtomicU32,
    Ordering,
};

/// Flags for the submission queue, written by the host, read by the guest.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SqFlags(pub u32);

impl SqFlags {
    /// Host has parked on epoll and needs a doorbell PIO write to wake up.
    pub const NEED_WAKEUP: Self = Self(1 << 0);
}

/// Flags for the completion queue, written by the guest, read by the host.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CqFlags(pub u32);

impl CqFlags {
    /// Guest is actively polling the CQ and does not need an interrupt.
    pub const NONE: Self = Self(0);
    /// Guest is sleeping and needs an interrupt injection to wake up.
    pub const NOTIFY_ME: Self = Self(1 << 0);
}

/// Shared control block at the start of the ring buffer region.
///
/// # Memory Ordering
///
/// On x86_64, `Relaxed` atomics compile to plain MOV instructions which are already sequentially
/// consistent for same-address accesses. We use `Release`/`Acquire` on the tail/head updates to
/// ensure payload visibility across the producer–consumer boundary.
#[repr(C, align(64))]
pub struct RingControl {
    // Submission queue indices.
    /// SQ head — advanced by the consumer (host/VMM) after draining entries.
    sq_head: AtomicU32,
    /// SQ tail — advanced by the producer (guest kernel) after posting entries.
    sq_tail: AtomicU32,
    /// SQ ring mask (ring size - 1). Immutable after init.
    sq_mask: u32,
    /// SQ flags (e.g., `NEED_WAKEUP`). Written by host, read by guest.
    sq_flags: AtomicU32,

    _pad0: [u8; 48], // Pad to separate cache line.

    // Completion queue indices.
    /// CQ head — advanced by the consumer (guest kernel) after reading entries.
    cq_head: AtomicU32,
    /// CQ tail — advanced by the producer (host/VMM) after posting entries.
    cq_tail: AtomicU32,
    /// CQ ring mask (ring size - 1). Immutable after init.
    cq_mask: u32,
    /// CQ flags (e.g., `NOTIFY_ME`). Written by guest, read by host.
    cq_flags: AtomicU32,

    _pad1: [u8; 48], // Pad to cache line boundary.
}

// Compile-time layout assertion.
const _: () = assert!(core::mem::size_of::<RingControl>() == 128);

impl RingControl {
    /// Initializes the control block for the given SQ and CQ sizes.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `sq_size` and `cq_size` are both powers of two.
    pub unsafe fn init(&mut self, sq_size: u32, cq_size: u32) {
        self.sq_head.store(0, Ordering::Relaxed);
        self.sq_tail.store(0, Ordering::Relaxed);
        self.sq_mask = sq_size - 1;
        self.sq_flags.store(SqFlags::NEED_WAKEUP.0, Ordering::Relaxed);
        self.cq_head.store(0, Ordering::Relaxed);
        self.cq_tail.store(0, Ordering::Relaxed);
        self.cq_mask = cq_size - 1;
        self.cq_flags.store(CqFlags::NOTIFY_ME.0, Ordering::Relaxed);
        self._pad0 = [0u8; 48];
        self._pad1 = [0u8; 48];
    }

    // -- Submission queue accessors --

    /// Returns the current SQ head (consumer position).
    #[inline]
    pub fn sq_head(&self) -> u32 {
        self.sq_head.load(Ordering::Acquire)
    }

    /// Returns the current SQ tail (producer position).
    #[inline]
    pub fn sq_tail(&self) -> u32 {
        self.sq_tail.load(Ordering::Acquire)
    }

    /// Returns the SQ ring mask.
    #[inline]
    pub fn sq_mask(&self) -> u32 {
        self.sq_mask
    }

    /// Advances the SQ head (consumer side) after draining entries.
    #[inline]
    pub fn advance_sq_head(&self, new_head: u32) {
        self.sq_head.store(new_head, Ordering::Release);
    }

    /// Advances the SQ tail (producer side) after posting entries.
    #[inline]
    pub fn advance_sq_tail(&self, new_tail: u32) {
        self.sq_tail.store(new_tail, Ordering::Release);
    }

    /// Returns the number of pending SQ entries.
    #[inline]
    pub fn sq_pending(&self) -> u32 {
        let tail: u32 = self.sq_tail();
        let head: u32 = self.sq_head();
        tail.wrapping_sub(head)
    }

    /// Returns `true` if the SQ is full.
    #[inline]
    pub fn sq_full(&self) -> bool {
        self.sq_pending() > self.sq_mask
    }

    /// Loads the SQ flags.
    #[inline]
    pub fn sq_flags(&self) -> SqFlags {
        SqFlags(self.sq_flags.load(Ordering::Acquire))
    }

    /// Stores new SQ flags (host side).
    #[inline]
    pub fn set_sq_flags(&self, flags: SqFlags) {
        self.sq_flags.store(flags.0, Ordering::Release);
    }

    // -- Completion queue accessors --

    /// Returns the current CQ head (consumer position).
    #[inline]
    pub fn cq_head(&self) -> u32 {
        self.cq_head.load(Ordering::Acquire)
    }

    /// Returns the current CQ tail (producer position).
    #[inline]
    pub fn cq_tail(&self) -> u32 {
        self.cq_tail.load(Ordering::Acquire)
    }

    /// Returns the CQ ring mask.
    #[inline]
    pub fn cq_mask(&self) -> u32 {
        self.cq_mask
    }

    /// Advances the CQ head (consumer side) after reading entries.
    #[inline]
    pub fn advance_cq_head(&self, new_head: u32) {
        self.cq_head.store(new_head, Ordering::Release);
    }

    /// Advances the CQ tail (producer side) after posting entries.
    #[inline]
    pub fn advance_cq_tail(&self, new_tail: u32) {
        self.cq_tail.store(new_tail, Ordering::Release);
    }

    /// Returns the number of pending CQ entries.
    #[inline]
    pub fn cq_pending(&self) -> u32 {
        let tail: u32 = self.cq_tail();
        let head: u32 = self.cq_head();
        tail.wrapping_sub(head)
    }

    /// Loads the CQ flags.
    #[inline]
    pub fn cq_flags(&self) -> CqFlags {
        CqFlags(self.cq_flags.load(Ordering::Acquire))
    }

    /// Stores new CQ flags (guest side).
    #[inline]
    pub fn set_cq_flags(&self, flags: CqFlags) {
        self.cq_flags.store(flags.0, Ordering::Release);
    }
}
