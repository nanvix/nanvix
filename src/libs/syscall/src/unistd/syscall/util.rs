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
mod tests {
    use super::*;
    use ::core::cell::Cell;

    /// Number of distinct pages a `len`-byte buffer starting at `ptr` touches. This is the upper
    /// bound on the number of scatter/gather segment descriptors the kernel builds for the chunk,
    /// because the kernel emits at most one descriptor per page it spans (contiguous physical pages
    /// are merged, so the real count is never larger).
    fn pages_spanned(ptr: usize, len: usize) -> usize {
        if len == 0 {
            return 0;
        }
        let page_offset: usize = ptr & (PAGE_SIZE - 1);
        (page_offset + len).div_ceil(PAGE_SIZE)
    }

    /// A zero-length request transfers nothing.
    #[test]
    fn zero_remaining_yields_zero() {
        assert_eq!(sg_chunk_size(0, 0), 0, "an empty request must produce an empty chunk");
        assert_eq!(
            sg_chunk_size(PAGE_SIZE - 1, 0),
            0,
            "an empty request must produce an empty chunk regardless of alignment"
        );
    }

    /// A buffer that already fits within the limits is transferred whole.
    #[test]
    fn small_buffer_is_not_split() {
        assert_eq!(sg_chunk_size(0, 100), 100, "a small page-aligned buffer must not be split");
        assert_eq!(
            sg_chunk_size(PAGE_SIZE - 10, 5),
            5,
            "a small buffer within a single page must not be split"
        );
    }

    /// A page-aligned buffer larger than the limit is capped at the maximum transfer size.
    #[test]
    fn page_aligned_large_buffer_caps_at_max_bytes() {
        assert_eq!(
            sg_chunk_size(0, SG_BULK_MAX_BYTES * 4),
            SG_BULK_MAX_BYTES,
            "a page-aligned buffer must be capped at the maximum scatter/gather transfer size"
        );
    }

    /// The chunk size must never describe more pages than the kernel can turn into descriptors in a
    /// single bounded heap allocation. This is the invariant that keeps the kernel's descriptor
    /// list within one heap slab; violating it reintroduces an unbounded allocation that the
    /// kernel-heap allocator rejects.
    #[test]
    fn chunk_never_exceeds_segment_budget() {
        let offsets: [usize; 6] = [0, 1, 37, PAGE_SIZE / 2, PAGE_SIZE - 100, PAGE_SIZE - 1];
        let remainings: [usize; 5] = [
            1,
            PAGE_SIZE,
            SG_BULK_MAX_BYTES - 1,
            SG_BULK_MAX_BYTES,
            SG_BULK_MAX_BYTES * 3,
        ];

        for &offset in &offsets {
            for &remaining in &remainings {
                let chunk: usize = sg_chunk_size(offset, remaining);

                assert!(
                    chunk > 0,
                    "a non-empty request must make progress (offset={offset}, \
                     remaining={remaining})"
                );
                assert!(
                    chunk <= remaining,
                    "a chunk must never exceed the requested length (offset={offset}, \
                     remaining={remaining}, chunk={chunk})"
                );
                assert!(
                    chunk <= SG_BULK_MAX_BYTES,
                    "a chunk must never exceed the maximum transfer size (offset={offset}, \
                     remaining={remaining}, chunk={chunk})"
                );
                assert!(
                    pages_spanned(offset, chunk) <= SG_BULK_MAX_SEGMENTS as usize,
                    "a chunk must never span more pages than the segment budget (offset={offset}, \
                     remaining={remaining}, chunk={chunk}, pages={}, max={SG_BULK_MAX_SEGMENTS})",
                    pages_spanned(offset, chunk)
                );
            }
        }
    }

    /// A `page_chunk_size` request that stays within a single page is transferred whole.
    #[test]
    fn page_chunk_within_single_page_is_not_split() {
        assert_eq!(
            page_chunk_size(0, 100),
            100,
            "a small page-aligned buffer must be transferred whole"
        );
        assert_eq!(
            page_chunk_size(PAGE_SIZE - 10, 5),
            5,
            "a small buffer that stays within one page must be transferred whole"
        );
    }

    /// A `page_chunk_size` request is capped at the end of the page that holds its first byte, so a
    /// chunk never crosses a page boundary. This is the property that lets the read loop treat a
    /// short reply as end-of-input rather than truncating a multi-page transfer.
    #[test]
    fn page_chunk_stops_at_page_boundary() {
        assert_eq!(
            page_chunk_size(0, PAGE_SIZE * 4),
            PAGE_SIZE,
            "a page-aligned multi-page request must be capped at a single page"
        );
        assert_eq!(
            page_chunk_size(PAGE_SIZE - 100, PAGE_SIZE * 4),
            100,
            "an unaligned request must be capped at the bytes left in its first page"
        );
    }

    /// Whatever the alignment and length, a `page_chunk_size` chunk must make forward progress and
    /// must never straddle a page boundary.
    #[test]
    fn page_chunk_never_crosses_a_page() {
        let offsets: [usize; 6] = [0, 1, 37, PAGE_SIZE / 2, PAGE_SIZE - 100, PAGE_SIZE - 1];
        let remainings: [usize; 5] = [1, 100, PAGE_SIZE - 1, PAGE_SIZE, PAGE_SIZE * 3 + 7];

        for &offset in &offsets {
            for &remaining in &remainings {
                let chunk: usize = page_chunk_size(offset, remaining);

                assert!(
                    chunk > 0,
                    "a non-empty request must make progress (offset={offset}, \
                     remaining={remaining})"
                );
                assert!(
                    chunk <= remaining,
                    "a chunk must never exceed the requested length (offset={offset}, \
                     remaining={remaining}, chunk={chunk})"
                );
                let page_offset: usize = offset & (PAGE_SIZE - 1);
                assert!(
                    page_offset + chunk <= PAGE_SIZE,
                    "a chunk must never cross a page boundary (offset={offset}, \
                     remaining={remaining}, chunk={chunk})"
                );
            }
        }
    }

    /// Builds the busy report vfsd returns while earlier I/O on the descriptor is in flight.
    fn in_progress() -> Error {
        Error::new(ErrorCode::OperationAlreadyInProgress, "earlier I/O is still in flight")
    }

    /// Builds a fake clock that starts at the epoch and advances one second per reading.
    fn ticking_clock(readings: &Cell<u64>) -> impl FnMut() -> Result<SystemTime, Error> + '_ {
        move || {
            let seconds: u64 = readings.get();
            readings.set(seconds + 1);
            Ok(SystemTime::new(seconds, 0).expect("fake clock reading must be valid"))
        }
    }

    /// Builds a wait hook that counts how many times the retry loop yielded.
    fn counting_wait(waits: &Cell<usize>) -> impl FnMut() -> Result<(), Error> + '_ {
        move || {
            waits.set(waits.get() + 1);
            Ok(())
        }
    }

    /// An uncontended request returns its outcome without reading the clock or waiting.
    #[test]
    fn retry_returns_first_outcome_without_reading_clock() {
        let readings: Cell<u64> = Cell::new(0);
        let waits: Cell<usize> = Cell::new(0);

        let result: Result<u32, Error> =
            retry_while_in_progress_with(|| Ok(7), ticking_clock(&readings), counting_wait(&waits));

        assert_eq!(result.expect("an uncontended request must succeed"), 7);
        assert_eq!(readings.get(), 0, "an uncontended request must not read the clock");
        assert_eq!(waits.get(), 0, "an uncontended request must not wait");
    }

    /// Errors other than the busy report are returned without retrying.
    #[test]
    fn retry_returns_other_errors_unchanged() {
        let attempts: Cell<usize> = Cell::new(0);
        let readings: Cell<u64> = Cell::new(0);
        let waits: Cell<usize> = Cell::new(0);

        let result: Result<(), Error> = retry_while_in_progress_with(
            || {
                attempts.set(attempts.get() + 1);
                Err(Error::new(ErrorCode::BadFile, "bad file descriptor"))
            },
            ticking_clock(&readings),
            counting_wait(&waits),
        );

        let error: Error = result.expect_err("a non-busy error must be returned");
        assert_eq!(error.code, ErrorCode::BadFile, "the original error must be preserved");
        assert_eq!(attempts.get(), 1, "a non-busy error must not be retried");
        assert_eq!(waits.get(), 0, "a non-busy error must not wait");
    }

    /// A busy request is retried until the earlier I/O drains.
    #[test]
    fn retry_waits_until_earlier_io_drains() {
        let attempts: Cell<usize> = Cell::new(0);
        let readings: Cell<u64> = Cell::new(0);
        let waits: Cell<usize> = Cell::new(0);

        let result: Result<u32, Error> = retry_while_in_progress_with(
            || {
                attempts.set(attempts.get() + 1);
                if attempts.get() < 3 {
                    Err(in_progress())
                } else {
                    Ok(3)
                }
            },
            ticking_clock(&readings),
            counting_wait(&waits),
        );

        assert_eq!(result.expect("the request must succeed once earlier I/O drains"), 3);
        assert_eq!(attempts.get(), 3, "each busy report must trigger exactly one retry");
        assert_eq!(waits.get(), 2, "the loop must yield before every retry");
    }

    /// A request fails with a timeout instead of retrying forever behind I/O that never drains.
    #[test]
    fn retry_times_out_when_earlier_io_never_drains() {
        let budget: usize =
            usize::try_from(IO_IN_PROGRESS_TIMEOUT.as_secs()).expect("retry budget must fit usize");
        let attempts: Cell<usize> = Cell::new(0);
        let readings: Cell<u64> = Cell::new(0);
        let waits: Cell<usize> = Cell::new(0);

        let result: Result<(), Error> = retry_while_in_progress_with(
            || {
                attempts.set(attempts.get() + 1);
                Err(in_progress())
            },
            ticking_clock(&readings),
            counting_wait(&waits),
        );

        let error: Error = result.expect_err("a request must not wait forever");
        assert_eq!(error.code, ErrorCode::OperationTimedOut, "an expired wait must time out");
        // The first busy report starts the budget, and the fake clock advances one second per
        // attempt, so the deadline is reached on the attempt after the budget is spent.
        assert_eq!(attempts.get(), budget + 1, "the request must stop once the budget is spent");
        assert_eq!(waits.get(), budget, "the loop must not yield after the deadline");
    }

    /// A failure to relinquish the processor aborts the retry.
    #[test]
    fn retry_propagates_wait_errors() {
        let readings: Cell<u64> = Cell::new(0);

        let result: Result<(), Error> = retry_while_in_progress_with(
            || Err(in_progress()),
            ticking_clock(&readings),
            || Err(Error::new(ErrorCode::Interrupted, "yield failed")),
        );

        let error: Error = result.expect_err("a wait failure must abort the retry");
        assert_eq!(error.code, ErrorCode::Interrupted, "the wait error must be preserved");
    }
}
