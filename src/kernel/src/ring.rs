// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Ring Buffer I/O
//!
//! Guest-side integration of the shared-memory ring buffer for batched syscall submission.
//! Replaces per-syscall PIO vmbus writes with SQE/CQE ring entries, reducing VM exits.

use ::alloc::collections::BTreeMap;
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
    FIXED_BUF_COUNT,
    FIXED_BUF_OFFSET,
    FIXED_BUF_SIZE,
    SQ_OFFSET,
    SQ_SIZE,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sys::pm::ThreadIdentifier;

/// Base virtual address of the ring buffer region (identity-mapped, equals GPA).
const RING_BASE: usize = ::config::microvm::RING_BUFFER_GPA;

/// Next data slot to use for guest-submitted request payloads.
static NEXT_DATA_SLOT: AtomicU32 = AtomicU32::new(0);
/// Per-thread fixed-buffer assignments.
static mut THREAD_FIXED_BUFFERS: BTreeMap<ThreadIdentifier, u32> = BTreeMap::new();
/// Allocation bitmap for fixed buffers.
static mut FIXED_BUFFER_IN_USE: [bool; FIXED_BUF_COUNT] = [false; FIXED_BUF_COUNT];

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

/// Returns the kernel virtual address of a fixed buffer in the shared ring region.
pub fn fixed_buffer_vaddr(buffer_id: u32) -> Result<usize, Error> {
    let buffer_index: usize = usize::try_from(buffer_id).map_err(|_| {
        let reason: &str = "invalid fixed buffer id";
        error!("{reason} (buffer_id={buffer_id})");
        Error::new(ErrorCode::InvalidArgument, reason)
    })?;
    if buffer_index >= FIXED_BUF_COUNT {
        let reason: &str = "fixed buffer id out of range";
        error!("{reason} (buffer_id={buffer_id}, fixed_buf_count={FIXED_BUF_COUNT})");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    Ok(RING_BASE + FIXED_BUF_OFFSET + buffer_index * FIXED_BUF_SIZE)
}

/// Returns the fixed buffer assigned to `caller_tid`, allocating one on first use.
pub fn get_or_alloc_thread_fixed_buffer(caller_tid: ThreadIdentifier) -> Result<u32, Error> {
    // SAFETY: Nanvix runs on a single core and accesses this state with interrupts disabled while
    // handling kernel calls and IKC completions.
    let thread_fixed_buffers: &mut BTreeMap<ThreadIdentifier, u32> =
        unsafe { &mut THREAD_FIXED_BUFFERS };
    if let Some(&buffer_id) = thread_fixed_buffers.get(&caller_tid) {
        return Ok(buffer_id);
    }

    // SAFETY: see note above.
    let fixed_buffer_in_use: &mut [bool; FIXED_BUF_COUNT] = unsafe { &mut FIXED_BUFFER_IN_USE };
    for (buffer_index, in_use) in fixed_buffer_in_use.iter_mut().enumerate() {
        if !*in_use {
            *in_use = true;
            let buffer_id: u32 = u32::try_from(buffer_index).map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, "fixed buffer index exceeds u32")
            })?;
            thread_fixed_buffers.insert(caller_tid, buffer_id);
            return Ok(buffer_id);
        }
    }

    let reason: &str = "no fixed ring buffers available";
    error!("{reason} (caller_tid={caller_tid:?})");
    Err(Error::new(ErrorCode::ResourceBusy, reason))
}

/// Releases the fixed buffer assigned to `caller_tid`, if any.
pub fn release_thread_fixed_buffer(caller_tid: ThreadIdentifier) {
    // SAFETY: Nanvix runs on a single core and accesses this state with interrupts disabled while
    // handling kernel calls and IKC completions.
    let thread_fixed_buffers: &mut BTreeMap<ThreadIdentifier, u32> =
        unsafe { &mut THREAD_FIXED_BUFFERS };
    let Some(buffer_id) = thread_fixed_buffers.remove(&caller_tid) else {
        return;
    };

    let buffer_index: usize = buffer_id as usize;
    if buffer_index >= FIXED_BUF_COUNT {
        error!(
            "fixed buffer id out of range while releasing thread buffer (caller_tid={caller_tid:?}, \
             buffer_id={buffer_id}, fixed_buf_count={FIXED_BUF_COUNT})"
        );
        return;
    }

    // SAFETY: see note above.
    let fixed_buffer_in_use: &mut [bool; FIXED_BUF_COUNT] = unsafe { &mut FIXED_BUFFER_IN_USE };
    fixed_buffer_in_use[buffer_index] = false;
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
