// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Ring Buffer I/O
//!
//! Guest-side integration of the shared-memory ring buffer for batched syscall submission.
//! Replaces per-syscall PIO vmbus writes with SQE/CQE ring entries, reducing VM exits.

use ::core::sync::atomic::{
    fence,
    AtomicU32,
    Ordering,
};
use ::nvx_ring::{
    CqFlags,
    CqEntry,
    RingControl,
    SqEntry,
    SqFlags,
    CONTROL_OFFSET,
    CQ_OFFSET,
    CQ_SIZE,
    DATA_OFFSET,
    DATA_SLOT_COUNT,
    DATA_SLOT_SIZE,
    SQ_OFFSET,
    SQ_SIZE,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

/// Base virtual address of the ring buffer region (identity-mapped, equals GPA).
const RING_BASE: usize = ::config::microvm::RING_BUFFER_GPA;

/// Next data slot to use for guest-submitted request payloads.
static NEXT_DATA_SLOT: AtomicU32 = AtomicU32::new(0);

/// Returns a mutable reference to the shared ring control block.
///
/// # Safety
///
/// The ring buffer region must be initialized and mapped at `RING_BASE`.
#[inline]
unsafe fn control() -> &'static RingControl {
    &*((RING_BASE + CONTROL_OFFSET) as *const RingControl)
}

/// Returns a mutable reference to the shared ring control block.
///
/// # Safety
///
/// The ring buffer region must be initialized and mapped at `RING_BASE`.
#[inline]
unsafe fn control_mut() -> &'static mut RingControl {
    &mut *((RING_BASE + CONTROL_OFFSET) as *mut RingControl)
}

/// Returns a pointer to the SQ entry array.
#[inline]
fn sq_entries() -> *mut SqEntry {
    (RING_BASE + SQ_OFFSET) as *mut SqEntry
}

/// Returns a pointer to the CQ entry array.
#[inline]
#[allow(dead_code)]
fn cq_entries() -> *const CqEntry {
    (RING_BASE + CQ_OFFSET) as *const CqEntry
}

/// Initializes the ring buffer control block.
///
/// Must be called once during kernel startup, before any ring I/O.
///
/// # Safety
///
/// The ring buffer GPA must be mapped and writable.
pub unsafe fn init() {
    let ctrl: &mut RingControl = control_mut();
    ctrl.init(SQ_SIZE, CQ_SIZE);
    info!("ring buffer initialized at GPA {:#010x}, SQ={}, CQ={}", RING_BASE, SQ_SIZE, CQ_SIZE);
}

/// Attempts to consume a single CQE from the completion queue.
#[inline]
unsafe fn try_consume_cqe(ctrl: &RingControl, entries: *const CqEntry) -> Option<CqEntry> {
    let head: u32 = ctrl.cq_head();
    let tail: u32 = ctrl.cq_tail();

    if head == tail {
        return None;
    }

    fence(Ordering::Acquire);
    let idx: usize = (head & ctrl.cq_mask()) as usize;
    let cqe: CqEntry = core::ptr::read_volatile(entries.add(idx));
    ctrl.advance_cq_head(head.wrapping_add(1));
    Some(cqe)
}

/// Submits a single SQE to the submission queue.
///
/// If the ring is full, spins until a slot becomes available.
/// After posting the entry, rings the doorbell if the host requested wakeup.
///
/// # Returns
///
/// The `user_data` tag of the submitted SQE (for matching with CQE).
pub fn submit(sqe: SqEntry) -> u64 {
    let user_data: u64 = sqe.user_data;

    // SAFETY: Ring buffer region is initialized and identity-mapped.
    unsafe {
        let ctrl: &RingControl = control();
        let entries: *mut SqEntry = sq_entries();

        // Spin-wait if the ring is full.
        loop {
            let head: u32 = ctrl.sq_head();
            let tail: u32 = ctrl.sq_tail();
            let pending: u32 = tail.wrapping_sub(head);
            if pending <= ctrl.sq_mask() {
                // Write entry at tail position.
                let idx: usize = (tail & ctrl.sq_mask()) as usize;
                core::ptr::write_volatile(entries.add(idx), sqe);

                // Publish the new tail.
                fence(Ordering::Release);
                ctrl.advance_sq_tail(tail.wrapping_add(1));

                // Ring doorbell if host needs wakeup.
                if ctrl.sq_flags() == SqFlags::NEED_WAKEUP {
                    ring_doorbell();
                }

                break;
            }
            core::hint::spin_loop();
        }
    }

    user_data
}

/// Polls the completion queue for a CQE matching the given `user_data` tag.
///
/// Blocks (spinning) until a matching CQE arrives.
#[allow(dead_code)]
pub fn poll(expected_user_data: u64) -> CqEntry {
    // SAFETY: Ring buffer region is initialized and identity-mapped.
    unsafe {
        let ctrl: &RingControl = control();
        let entries: *const CqEntry = cq_entries();

        loop {
            let head: u32 = ctrl.cq_head();
            let tail: u32 = ctrl.cq_tail();

            if head != tail {
                fence(Ordering::Acquire);
                let idx: usize = (head & ctrl.cq_mask()) as usize;
                let cqe: CqEntry = core::ptr::read_volatile(entries.add(idx));

                // Advance head.
                ctrl.advance_cq_head(head.wrapping_add(1));

                if cqe.user_data == expected_user_data {
                    return cqe;
                }
                // Non-matching CQE — this shouldn't happen in single-thread mode,
                // but continue polling.
            }
            core::hint::spin_loop();
        }
    }
}

/// Polls the CQ without blocking and arms `CQ_NOTIFY_ME` when it becomes empty.
///
/// The queue is re-checked after arming the notification flag to avoid a lost wakeup race if the
/// host posts a CQE between the empty check and the flag store.
pub fn try_poll_or_enable_notification() -> Option<CqEntry> {
    // SAFETY: Ring buffer region is initialized and identity-mapped.
    unsafe {
        let ctrl: &RingControl = control();
        let entries: *const CqEntry = cq_entries();

        ctrl.set_cq_flags(CqFlags::NONE);
        if let Some(cqe) = try_consume_cqe(ctrl, entries) {
            return Some(cqe);
        }

        ctrl.set_cq_flags(CqFlags::NOTIFY_ME);
        fence(Ordering::SeqCst);

        if let Some(cqe) = try_consume_cqe(ctrl, entries) {
            ctrl.set_cq_flags(CqFlags::NONE);
            return Some(cqe);
        }

        None
    }
}

/// Returns a raw pointer to a data slot in the ring buffer data region.
///
/// # Safety
///
/// The caller must ensure `slot_id < DATA_SLOT_COUNT`.
#[inline]
pub unsafe fn data_slot_ptr(slot_id: u32) -> *const u8 {
    (RING_BASE + DATA_OFFSET + (slot_id as usize) * DATA_SLOT_SIZE) as *const u8
}

/// Copies a request payload into a shared data slot and returns its GPA.
pub fn write_data(bytes: &[u8]) -> Result<u64, Error> {
    if bytes.len() > DATA_SLOT_SIZE {
        let reason: &str = "ring request payload too large";
        error!("{reason} (len={}, max={})", bytes.len(), DATA_SLOT_SIZE);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    let slot_id: u32 = NEXT_DATA_SLOT.fetch_add(1, Ordering::Relaxed) % (DATA_SLOT_COUNT as u32);
    let slot_addr: usize = RING_BASE + DATA_OFFSET + (slot_id as usize) * DATA_SLOT_SIZE;

    // SAFETY: The ring buffer data region is reserved and identity-mapped by the kernel.
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), slot_addr as *mut u8, bytes.len());
    }

    Ok(slot_addr as u64)
}

/// Rings the doorbell by writing to the doorbell PIO port.
#[inline]
fn ring_doorbell() {
    // SAFETY: The doorbell port is a valid PIO port registered with the VMM.
    unsafe {
        ::arch::io::out32(::config::microvm::RING_DOORBELL_PORT, 1);
    }
}
