// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Type-safe wrappers for producer and consumer access to a ring.
//!
//! These are zero-cost wrappers that enforce the SPSC discipline: the producer only writes the
//! tail and entries, and the consumer only writes the head and reads entries. Both sides read
//! the other's index for availability checks.

use crate::{
    cqe::CqEntry,
    sqe::SqEntry,
};
use core::sync::atomic::{
    fence,
    Ordering,
};

/// Producer-side handle for posting entries to a ring.
///
/// For the SQ, the guest kernel is the producer.
/// For the CQ, the host VMM is the producer.
pub struct RingProducer<'a, T: Copy> {
    entries: &'a mut [T],
    mask: u32,
    // We store a local copy of the tail to avoid repeated atomic loads.
    cached_tail: u32,
}

impl<'a, T: Copy> RingProducer<'a, T> {
    /// Creates a new producer handle.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `entries` points to a valid ring buffer array
    /// and that `mask` equals `ring_size - 1` where `ring_size` is a power of two.
    pub unsafe fn new(entries: &'a mut [T], mask: u32, initial_tail: u32) -> Self {
        Self {
            entries,
            mask,
            cached_tail: initial_tail,
        }
    }

    /// Returns the index into the entry array for the given sequence number.
    #[inline]
    fn slot(&self, seq: u32) -> usize {
        (seq & self.mask) as usize
    }

    /// Posts one entry to the ring. Returns `false` if the ring is full.
    ///
    /// The caller must call [`Self::flush`] afterwards to make entries visible.
    #[inline]
    pub fn push(&mut self, entry: T, head: u32) -> bool {
        let pending: u32 = self.cached_tail.wrapping_sub(head);
        if pending > self.mask {
            return false; // Ring full.
        }
        let idx: usize = self.slot(self.cached_tail);
        self.entries[idx] = entry;
        self.cached_tail = self.cached_tail.wrapping_add(1);
        true
    }

    /// Returns the current local tail (not yet flushed).
    #[inline]
    pub fn tail(&self) -> u32 {
        self.cached_tail
    }

    /// Publishes all pushed entries by storing the new tail with release ordering.
    #[inline]
    pub fn flush(&self, ctrl_tail_store: impl FnOnce(u32)) {
        // Ensure all entry writes are visible before the tail update.
        fence(Ordering::Release);
        ctrl_tail_store(self.cached_tail);
    }
}

/// Consumer-side handle for draining entries from a ring.
///
/// For the SQ, the host VMM is the consumer.
/// For the CQ, the guest kernel is the consumer.
pub struct RingConsumer<'a, T: Copy> {
    entries: &'a [T],
    mask: u32,
    cached_head: u32,
}

impl<'a, T: Copy> RingConsumer<'a, T> {
    /// Creates a new consumer handle.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `entries` points to a valid ring buffer array
    /// and that `mask` equals `ring_size - 1` where `ring_size` is a power of two.
    pub unsafe fn new(entries: &'a [T], mask: u32, initial_head: u32) -> Self {
        Self {
            entries,
            mask,
            cached_head: initial_head,
        }
    }

    /// Returns the index into the entry array for the given sequence number.
    #[inline]
    fn slot(&self, seq: u32) -> usize {
        (seq & self.mask) as usize
    }

    /// Pops one entry from the ring. Returns `None` if the ring is empty.
    ///
    /// The caller must call [`Self::flush`] afterwards to make head advancement visible.
    #[inline]
    pub fn pop(&mut self, tail: u32) -> Option<T> {
        if self.cached_head == tail {
            return None; // Ring empty.
        }
        // Ensure we read the entry after observing the tail update.
        fence(Ordering::Acquire);
        let idx: usize = self.slot(self.cached_head);
        let entry: T = self.entries[idx];
        self.cached_head = self.cached_head.wrapping_add(1);
        Some(entry)
    }

    /// Returns the current local head (not yet flushed).
    #[inline]
    pub fn head(&self) -> u32 {
        self.cached_head
    }

    /// Returns the number of entries available to consume.
    #[inline]
    pub fn available(&self, tail: u32) -> u32 {
        tail.wrapping_sub(self.cached_head)
    }

    /// Publishes the new head position so the producer can reclaim slots.
    #[inline]
    pub fn flush(&self, ctrl_head_store: impl FnOnce(u32)) {
        ctrl_head_store(self.cached_head);
    }
}

// Convenience type aliases for the two rings.

/// SQ producer (guest kernel side).
#[allow(dead_code)]
pub type SqProducer<'a> = RingProducer<'a, SqEntry>;

/// SQ consumer (host VMM side).
#[allow(dead_code)]
pub type SqConsumer<'a> = RingConsumer<'a, SqEntry>;

/// CQ producer (host VMM side).
#[allow(dead_code)]
pub type CqProducer<'a> = RingProducer<'a, CqEntry>;

/// CQ consumer (guest kernel side).
#[allow(dead_code)]
pub type CqConsumer<'a> = RingConsumer<'a, CqEntry>;
