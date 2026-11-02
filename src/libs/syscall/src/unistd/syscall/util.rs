// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_SIZE;
use ::core::{
    cmp,
    time::Duration,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        SG_BULK_MAX_BYTES,
        SG_BULK_MAX_SEGMENTS,
    },
    time::SystemTime,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum time a request waits for earlier offset-sensitive I/O on its open file description.
///
/// vfsd serializes HostFS reads, writes, and seeks per open file description and rejects a request
/// with [`ErrorCode::OperationAlreadyInProgress`] while an earlier one, including one abandoned by
/// cancellation or process exit, still awaits its host response. Bounding the retry keeps a host
/// that never answers from stalling later I/O on the descriptor forever.
const IO_IN_PROGRESS_TIMEOUT: Duration = Duration::from_secs(5);

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Retries `operation` while it reports earlier offset-sensitive I/O on the same open file
/// description. The clock is read only after the first busy report, so uncontended requests pay
/// no extra kernel calls.
///
/// # Parameters
///
/// - `operation`: Issues one request and returns its outcome.
/// - `now`: Reads the current time.
/// - `wait`: Relinquishes the processor before the next attempt.
///
/// # Returns
///
/// Upon successful completion, the first outcome of `operation` that is not a busy report is
/// returned. Otherwise, an error is returned instead.
///
/// # Errors
///
/// - [`ErrorCode::OperationTimedOut`]: The earlier I/O did not drain within
///   [`IO_IN_PROGRESS_TIMEOUT`].
/// - [`ErrorCode::ValueOutOfRange`]: The retry deadline overflows the clock.
/// - Any error reported by `now` or `wait`.
///
pub fn retry_while_in_progress_with<T>(
    mut operation: impl FnMut() -> Result<T, Error>,
    mut now: impl FnMut() -> Result<SystemTime, Error>,
    mut wait: impl FnMut() -> Result<(), Error>,
) -> Result<T, Error> {
    let mut deadline: Option<SystemTime> = None;
    loop {
        match operation() {
            Err(error) if error.code == ErrorCode::OperationAlreadyInProgress => {},
            result => return result,
        }

        let current: SystemTime = now()?;
        let expiry: SystemTime = match deadline {
            Some(expiry) => expiry,
            None => match current.checked_add_duration(&IO_IN_PROGRESS_TIMEOUT) {
                Some(expiry) => {
                    deadline = Some(expiry);
                    expiry
                },
                None => {
                    let reason: &str = "I/O retry deadline overflow";
                    #[cfg(feature = "syscall")]
                    ::syslog::warn!("retry_while_in_progress_with(): {reason} (now={current:?})");
                    return Err(Error::new(ErrorCode::ValueOutOfRange, reason));
                },
            },
        };
        if current >= expiry {
            let reason: &str = "earlier I/O on the open file description did not complete";
            #[cfg(feature = "syscall")]
            ::syslog::warn!(
                "retry_while_in_progress_with(): {reason} (timeout={IO_IN_PROGRESS_TIMEOUT:?})"
            );
            return Err(Error::new(ErrorCode::OperationTimedOut, reason));
        }
        wait()?;
    }
}

///
/// # Description
///
/// Retries a VFS request while earlier offset-sensitive I/O on the same open file description is
/// in flight, yielding the processor between attempts.
///
/// # Parameters
///
/// - `operation`: Issues one request and returns its outcome.
///
/// # Returns
///
/// Upon successful completion, the first outcome of `operation` that is not a busy report is
/// returned. Otherwise, an error is returned instead.
///
/// # Errors
///
/// See [`retry_while_in_progress_with`].
///
#[cfg(feature = "syscall")]
pub fn retry_while_in_progress<T>(operation: impl FnMut() -> Result<T, Error>) -> Result<T, Error> {
    retry_while_in_progress_with(
        operation,
        || {
            let mut now: SystemTime = SystemTime::default();
            ::sys::kcall::pm::__kcall_gettime(&mut now)?;
            Ok(now)
        },
        ::sys::kcall::sched::__kcall_sched_yield,
    )
}

///
/// # Description
///
/// Computes the number of bytes that can be represented by a single scatter/gather bulk transfer.
///
/// # Parameters
///
/// - `ptr`: Start address of the buffer.
/// - `remaining`: Total number of bytes remaining to transfer.
///
/// # Returns
///
/// The number of bytes that fit within the scatter/gather limits.
///
pub fn sg_chunk_size(ptr: usize, remaining: usize) -> usize {
    if remaining == 0 {
        return 0;
    }

    let page_offset: usize = ptr & (PAGE_SIZE - 1);
    let first_page_bytes: usize = PAGE_SIZE - page_offset;
    let max_by_segments: usize = if SG_BULK_MAX_SEGMENTS == 0 {
        0
    } else {
        first_page_bytes + (SG_BULK_MAX_SEGMENTS as usize - 1) * PAGE_SIZE
    };

    cmp::min(remaining, cmp::min(SG_BULK_MAX_BYTES, max_by_segments))
}

///
/// # Description
///
/// Computes the number of bytes that can be transferred starting at `ptr` without crossing a page
/// boundary.
///
/// Unlike [`sg_chunk_size`], this caps a chunk at a single page. The read path uses it because the
/// IKC backends deliver at most one page per request: vfsd reads into a one-page bulk buffer, and
/// the standalone stdin bridge returns only the bytes currently available. With page-sized requests
/// a short reply unambiguously signals end-of-input -- true EOF for files, "no more bytes ready"
/// for streams -- so the read loop can stop without truncating a multi-page file or blocking a
/// partially-filled stream. A multi-page request would instead be capped by the backend, and the
/// resulting short reply would be indistinguishable from EOF. The write path has no such ambiguity
/// (the whole source buffer is already available, so it loops until no forward progress is made)
/// and therefore keeps using [`sg_chunk_size`].
///
/// # Parameters
///
/// - `ptr`: Start address of the buffer.
/// - `remaining`: Total number of bytes remaining to transfer.
///
/// # Returns
///
/// The number of bytes that fit within the current page.
///
pub fn page_chunk_size(ptr: usize, remaining: usize) -> usize {
    let page_offset: usize = ptr & (PAGE_SIZE - 1);
    let available: usize = PAGE_SIZE - page_offset;
    cmp::min(available, remaining)
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests;
