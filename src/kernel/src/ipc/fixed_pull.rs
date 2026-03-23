// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::VirtualAddress,
    pm::{
        sync::condvar::Condvar,
        ProcessManager,
        SleepError,
    },
};
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    sync::Arc,
};
use core::sync::atomic::{
    AtomicI32,
    AtomicUsize,
    Ordering,
};
use sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Ordering used for all atomic operations. Relaxed is safe because Nanvix is a single-core system
/// and the kernel runs with interrupts disabled.
const ORDER: Ordering = Ordering::Relaxed;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Clone, Copy)]
pub struct FixedPullSegment {
    buffer_id: u32,
    user_offset: usize,
    buffer_len: usize,
}

impl FixedPullSegment {
    pub const fn new(buffer_id: u32, user_offset: usize, buffer_len: usize) -> Self {
        Self {
            buffer_id,
            user_offset,
            buffer_len,
        }
    }

    const fn zeroed() -> Self {
        Self {
            buffer_id: 0,
            user_offset: 0,
            buffer_len: 0,
        }
    }
}

struct PendingFixedPull {
    condvar: Condvar,
    bytes_transferred: Arc<AtomicUsize>,
    status_code: Arc<AtomicI32>,
    caller_pid: ProcessIdentifier,
    buffer_raw: usize,
    segments: [FixedPullSegment; crate::ring::MAX_FIXED_BUFFERS_PER_TRANSFER],
    segment_count: usize,
}

//==================================================================================================
// Global Variables
//==================================================================================================

/// Pending fixed-buffer pull requests keyed by thread identifier.
static mut PENDING_FIXED_PULLS: BTreeMap<ThreadIdentifier, Box<PendingFixedPull>> = BTreeMap::new();

//==================================================================================================
// Internal Helpers
//==================================================================================================

fn copy_segment_to_user(
    entry: &PendingFixedPull,
    segment: FixedPullSegment,
    data_len: usize,
) -> Result<(), Error> {
    if data_len == 0 {
        return Ok(());
    }

    let src: VirtualAddress =
        VirtualAddress::from_raw_value(crate::ring::fixed_buffer_vaddr(segment.buffer_id)?);
    let dst: VirtualAddress =
        VirtualAddress::from_raw_value(entry.buffer_raw + segment.user_offset);
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
    pm.vmcopy_to_user(entry.caller_pid, dst, src, data_len)
}

fn wake_pending_pull(
    caller_tid: ThreadIdentifier,
    entry: Box<PendingFixedPull>,
    bytes_transferred: usize,
    batched: bool,
) -> bool {
    // SAFETY: the calling process does not hold a reference to the process manager.
    if let Err(error) = unsafe { entry.condvar.notify_thread(caller_tid) } {
        error!(
            "wake_pending_pull(): failed to wake up sleeping fixed pull thread \
             (tid={caller_tid:?}, error={error:?})"
        );
    }

    let completion_kind: &str = if batched { "batched" } else { "segmented" };
    trace!(
        "fixed pull {completion_kind} completion finished (caller_tid={caller_tid:?}, \
         bytes_transferred={bytes_transferred})"
    );

    true
}

fn complete_batched(
    pending: &mut BTreeMap<ThreadIdentifier, Box<PendingFixedPull>>,
    caller_tid: ThreadIdentifier,
    data_len: usize,
) -> bool {
    let Some(entry) = pending.remove(&caller_tid) else {
        warn!("complete_batched(): no pending fixed pull found for tid={caller_tid:?}");
        return false;
    };

    let announced_len: usize = entry.segments[..entry.segment_count]
        .iter()
        .fold(0usize, |total, segment| total.saturating_add(segment.buffer_len));
    if data_len > announced_len {
        warn!(
            "complete_batched(): completion length exceeds announced buffers \
             (caller_tid={caller_tid:?}, data_len={data_len}, announced_len={announced_len})"
        );
    }

    let mut remaining: usize = core::cmp::min(data_len, announced_len);
    let mut bytes_copied: usize = 0;

    for segment in entry.segments[..entry.segment_count].iter().copied() {
        if remaining == 0 {
            break;
        }

        let bytes_to_copy: usize = core::cmp::min(remaining, segment.buffer_len);
        match copy_segment_to_user(&entry, segment, bytes_to_copy) {
            Ok(()) => {
                bytes_copied += bytes_to_copy;
                remaining -= bytes_to_copy;
            },
            Err(error) => {
                error!(
                    "complete_batched(): failed to copy fixed-buffer payload to user buffer \
                     (caller_tid={caller_tid:?}, caller_pid={:?}, buffer_id={}, buffer_raw={:#x}, \
                     data_len={bytes_to_copy}, error={error:?})",
                    entry.caller_pid, segment.buffer_id, entry.buffer_raw
                );
                entry.status_code.store(i32::from(error.code), ORDER);
                break;
            },
        }
    }

    entry.bytes_transferred.store(bytes_copied, ORDER);
    wake_pending_pull(caller_tid, entry, bytes_copied, true)
}

//==================================================================================================
// Public Functions
//==================================================================================================

pub fn register_and_sleep(
    caller_pid: ProcessIdentifier,
    caller_tid: ThreadIdentifier,
    buffer_raw: usize,
    segments: &[FixedPullSegment],
) -> Result<usize, SleepError> {
    if segments.is_empty() {
        let reason: &str = "fixed pull requires at least one segment";
        error!("{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?})");
        return Err(SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason)));
    }
    if segments.len() > crate::ring::MAX_FIXED_BUFFERS_PER_TRANSFER {
        let reason: &str = "fixed pull exceeds maximum segment count";
        error!(
            "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, segments={})",
            segments.len()
        );
        return Err(SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason)));
    }

    let condvar: Condvar = Condvar::new();
    let bytes_transferred: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let status_code: Arc<AtomicI32> = Arc::new(AtomicI32::new(0));

    let condvar_clone: Condvar = condvar.clone();
    let bytes_transferred_clone: Arc<AtomicUsize> = bytes_transferred.clone();
    let status_code_clone: Arc<AtomicI32> = status_code.clone();

    // SAFETY: single-core system with interrupts disabled.
    let pending: &mut BTreeMap<ThreadIdentifier, Box<PendingFixedPull>> =
        unsafe { &mut PENDING_FIXED_PULLS };
    let mut stored_segments: [FixedPullSegment; crate::ring::MAX_FIXED_BUFFERS_PER_TRANSFER] =
        [FixedPullSegment::zeroed(); crate::ring::MAX_FIXED_BUFFERS_PER_TRANSFER];
    stored_segments[..segments.len()].copy_from_slice(segments);
    pending.insert(
        caller_tid,
        Box::new(PendingFixedPull {
            condvar: condvar_clone,
            bytes_transferred: bytes_transferred_clone,
            status_code: status_code_clone,
            caller_pid,
            buffer_raw,
            segments: stored_segments,
            segment_count: segments.len(),
        }),
    );

    trace!(
        "fixed pull sleeping (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
         buffer_raw={buffer_raw:#x}, segment_count={})",
        segments.len()
    );

    // SAFETY: no global resources are held, the calling thread is not the kernel.
    match unsafe { condvar.wait(None) } {
        Ok(()) => {
            let status: i32 = status_code.load(ORDER);
            if status != 0 {
                let error_code: ErrorCode =
                    ErrorCode::try_from(i64::from(status)).unwrap_or_else(|_| {
                        error!(
                            "register_and_sleep(): invalid completion error code \
                             (status={status}), falling back to InvalidMessage"
                        );
                        ErrorCode::InvalidMessage
                    });
                return Err(SleepError::Generic(Error::new(
                    error_code,
                    "fixed pull completion failed",
                )));
            }

            let actual: usize = bytes_transferred.load(ORDER);
            Ok(actual)
        },
        Err(error) => {
            // SAFETY: single-core system with interrupts disabled.
            let pending: &mut BTreeMap<ThreadIdentifier, Box<PendingFixedPull>> =
                unsafe { &mut PENDING_FIXED_PULLS };
            pending.remove(&caller_tid);
            Err(error)
        },
    }
}

pub fn complete(
    caller_tid: ThreadIdentifier,
    buffer_id: u32,
    data_len: usize,
    more: bool,
    batched: bool,
) -> bool {
    // SAFETY: single-core system with interrupts disabled.
    let pending: &mut BTreeMap<ThreadIdentifier, Box<PendingFixedPull>> =
        unsafe { &mut PENDING_FIXED_PULLS };

    if batched {
        return complete_batched(pending, caller_tid, data_len);
    }

    let Some(entry) = pending.get_mut(&caller_tid) else {
        warn!("complete(): no pending fixed pull found for tid={caller_tid:?}");
        return false;
    };

    let Some(segment) = entry.segments[..entry.segment_count]
        .iter()
        .find(|segment| segment.buffer_id == buffer_id)
        .copied()
    else {
        warn!(
            "complete(): no matching fixed pull segment found for tid={caller_tid:?}, \
             buffer_id={buffer_id}"
        );
        return false;
    };

    let bytes_to_copy: usize = core::cmp::min(data_len, segment.buffer_len);
    let copy_result: Result<(), Error> = copy_segment_to_user(entry, segment, bytes_to_copy);

    match copy_result {
        Ok(()) => {
            entry.bytes_transferred.fetch_add(bytes_to_copy, ORDER);
        },
        Err(error) => {
            error!(
                "complete(): failed to copy fixed-buffer payload to user buffer \
                 (caller_tid={caller_tid:?}, caller_pid={:?}, buffer_id={buffer_id}, \
                 buffer_raw={:#x}, data_len={data_len}, error={error:?})",
                entry.caller_pid, entry.buffer_raw
            );
            entry.status_code.store(i32::from(error.code), ORDER);
        },
    }

    if more {
        trace!(
            "fixed pull segment completed (caller_tid={caller_tid:?}, buffer_id={buffer_id}, \
             bytes_transferred={bytes_to_copy})"
        );
        return true;
    }

    let total_bytes_transferred: usize = entry.bytes_transferred.load(ORDER);
    let Some(entry) = pending.remove(&caller_tid) else {
        warn!("complete(): missing fixed pull entry while finishing tid={caller_tid:?}");
        return false;
    };
    wake_pending_pull(caller_tid, entry, total_bytes_transferred, false)
}
