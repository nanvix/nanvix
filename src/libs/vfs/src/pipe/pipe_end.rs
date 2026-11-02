// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! In-memory pipe buffers for POSIX unnamed pipes.
//!
//! A pipe is a fixed-capacity byte ring buffer plus reader/writer reference counts. The buffer is
//! owned by the VFS library (`no_std`) so that it lives next to the existing FD machinery; the
//! daemon (vfsd) holds no buffer bytes and drives all blocking from its side via the non-blocking
//! primitives exposed here.
//!
//! # Model
//!
//! [`PipeEnd::new_pair`] allocates one [`PipeInner`] (shared by both ends) and returns the read end
//! and write end. Each end is stored inside a `VfsFileHandle::Pipe` in its own open file
//! description (`Arc<Mutex<VfsEntry>>`). `fork()` clones the descriptor slots, sharing the same
//! end, so the reader/writer counts are unaffected — matching POSIX, where `fork()` shares the open
//! file description rather than creating a new one.
//!
//! [`PipeEnd`] implements [`Drop`]: when the last descriptor referring to a given end's open file
//! description is closed (or its owning process exits), the description drops, [`PipeEnd::drop`]
//! runs, and decrements `readers` or `writers` on the shared [`PipeInner`]. The corresponding
//! *wakeup* of suspended callers is the daemon's responsibility (the VFS only adjusts the count).

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    PipeEndError,
    PipeReadOutcome,
    PipeWriteOutcome,
};
use ::alloc::{
    collections::{
        vec_deque::Drain,
        VecDeque,
    },
    sync::Arc,
};
use ::core::sync::atomic::{
    AtomicU64,
    Ordering,
};
use ::spin::Mutex;
use ::sysapi::{
    ffi::c_short,
    poll::{
        poll_errors::{
            POLLERR,
            POLLHUP,
        },
        poll_flags::{
            POLLIN,
            POLLOUT,
            POLLRDNORM,
            POLLWRNORM,
        },
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Capacity of a pipe's kernel buffer, in bytes.
///
/// Matches the Linux default and is guaranteed to be at least [`PIPE_BUF`].
pub const PIPE_CAPACITY: usize = 64 * 1024;

/// Maximum size of an atomic pipe write (POSIX `PIPE_BUF`).
///
/// A write of at most this many bytes is performed all-or-nothing and never interleaves with the
/// data of another writer. POSIX requires this to be at least 512.
pub const PIPE_BUF: usize = 4096;

//==================================================================================================
// Pipe Identity Allocator
//==================================================================================================

/// Source of process-global, monotonically increasing pipe identifiers.
///
/// Identifiers are never recycled for the lifetime of the daemon, so a stale wakeup can never
/// target the wrong pipe. Starts at `1` so that `0` can be reserved as an always-invalid sentinel
/// by callers that need one.
static NEXT_PIPE_ID: AtomicU64 = AtomicU64::new(1);

/// Allocates the next unique pipe identifier.
fn alloc_pipe_id() -> u64 {
    // `fetch_add` returns the previous value, so on wraparound it can yield `0`. Skip it to keep
    // `0` reserved as an always-invalid sentinel.
    loop {
        let id: u64 = NEXT_PIPE_ID.fetch_add(1, Ordering::Relaxed);
        if id != 0 {
            return id;
        }
    }
}

//==================================================================================================
// Pipe Buffer
//==================================================================================================

/// Shared state of a pipe: the byte ring buffer plus reader/writer reference counts.
struct PipeInner {
    /// Byte ring buffer, capped at [`PIPE_CAPACITY`].
    buf: VecDeque<u8>,
    /// Number of open read-end descriptions (`0` or `1` under `pipe()`/`dup`/`fork` sharing).
    readers: u32,
    /// Number of open write-end descriptions.
    writers: u32,
}

impl PipeInner {
    /// Non-destructively copies bytes from the front of the buffer.
    fn peek(&self, buf: &mut [u8]) -> PipeReadOutcome {
        // A zero-length read must succeed immediately and never block, even on an empty pipe.
        if buf.is_empty() {
            return PipeReadOutcome::Read(0);
        }
        let available: usize = self.buf.len();
        if available == 0 {
            if self.writers == 0 {
                return PipeReadOutcome::Eof;
            }
            return PipeReadOutcome::WouldBlock;
        }
        let n: usize = available.min(buf.len());
        for (dst, src) in buf.iter_mut().zip(self.buf.iter().take(n)) {
            *dst = *src;
        }
        PipeReadOutcome::Read(n)
    }

    /// Removes up to `count` bytes from the front of the buffer.
    fn consume(&mut self, count: usize) -> usize {
        let consumed: usize = count.min(self.buf.len());
        let _dropped: Drain<'_, u8> = self.buf.drain(..consumed);
        consumed
    }

    /// Non-blocking read from the buffer.
    ///
    /// Copies as many bytes as are available into `buf` (up to `buf.len()`). Returns
    /// [`PipeReadOutcome::WouldBlock`] when the buffer is empty but at least one writer is still
    /// open, and [`PipeReadOutcome::Eof`] when the buffer is empty and no writers remain. A
    /// zero-length read transfers no data and never blocks — POSIX `read(2)` with `count == 0`
    /// returns `0` immediately.
    fn read(&mut self, buf: &mut [u8]) -> PipeReadOutcome {
        let outcome: PipeReadOutcome = self.peek(buf);
        if let PipeReadOutcome::Read(n) = outcome {
            let consumed: usize = self.consume(n);
            debug_assert_eq!(consumed, n);
        }
        outcome
    }

    /// Non-blocking write into the buffer, honoring [`PIPE_BUF`] atomicity.
    ///
    /// If `buf.len() <= PIPE_BUF` the write is all-or-nothing: it either appends every byte or,
    /// when there is insufficient free space, appends nothing and returns
    /// [`PipeWriteOutcome::WouldBlock`]. For larger writes, as much as fits is appended. Returns
    /// [`PipeWriteOutcome::BrokenPipe`] when no readers remain.
    fn write(&mut self, buf: &[u8]) -> PipeWriteOutcome {
        // A zero-length write transfers no data and has no other effect, so it succeeds even with
        // no readers — matching Linux, which never reports `EPIPE`/`SIGPIPE` for an empty write.
        if buf.is_empty() {
            return PipeWriteOutcome::Wrote(0);
        }
        if self.readers == 0 {
            return PipeWriteOutcome::BrokenPipe;
        }
        let free: usize = PIPE_CAPACITY.saturating_sub(self.buf.len());
        let to_write: usize = if buf.len() <= PIPE_BUF {
            // Atomic write: all-or-nothing.
            if free < buf.len() {
                return PipeWriteOutcome::WouldBlock;
            }
            buf.len()
        } else {
            // Large write: append as much as fits.
            if free == 0 {
                return PipeWriteOutcome::WouldBlock;
            }
            free.min(buf.len())
        };
        self.buf.extend(buf[..to_write].iter().copied());
        PipeWriteOutcome::Wrote(to_write)
    }
}

//==================================================================================================
// Pipe End
//==================================================================================================

/// One end of a pipe, stored inside a `VfsFileHandle::Pipe`.
///
/// Both ends share the same [`PipeInner`] through an [`Arc`]; the `is_write` flag distinguishes the
/// write end from the read end and is used to enforce I/O direction.
pub struct PipeEnd {
    inner: Arc<Mutex<PipeInner>>,
    is_write: bool,
    /// Stable identity used by vfsd to key blocked-request queues.
    ///
    /// Copied onto each end at construction so that identity lookups (and the `Drop`/exit paths
    /// that report it) never need to take the shared [`PipeInner`] lock.
    pipe_id: u64,
    /// Wall-clock creation time (Unix seconds), reported for all three stat times.
    created: i64,
}

impl PipeEnd {
    /// Allocates a new pipe and returns its `(read_end, write_end)` pair.
    ///
    /// The shared [`PipeInner`] starts with `readers == 1` and `writers == 1`.
    pub fn new_pair() -> (PipeEnd, PipeEnd) {
        let pipe_id: u64 = alloc_pipe_id();
        let created: i64 = crate::time::wall_clock_secs();
        let inner: Arc<Mutex<PipeInner>> = Arc::new(Mutex::new(PipeInner {
            buf: VecDeque::new(),
            readers: 1,
            writers: 1,
        }));
        let read_end: PipeEnd = PipeEnd {
            inner: inner.clone(),
            is_write: false,
            pipe_id,
            created,
        };
        let write_end: PipeEnd = PipeEnd {
            inner,
            is_write: true,
            pipe_id,
            created,
        };
        (read_end, write_end)
    }

    /// Returns the stable identity of the pipe this end belongs to.
    pub fn pipe_id(&self) -> u64 {
        self.pipe_id
    }

    /// Returns the wall-clock creation time (Unix seconds) of this pipe.
    pub fn created(&self) -> i64 {
        self.created
    }

    /// Returns `true` if this is the write end, `false` if it is the read end.
    pub fn is_write(&self) -> bool {
        self.is_write
    }

    /// Returns the events that can complete immediately on this pipe end.
    pub fn poll(&self, events: c_short) -> c_short {
        const READ_EVENTS: c_short = POLLIN | POLLRDNORM;
        const WRITE_EVENTS: c_short = POLLOUT | POLLWRNORM;

        let inner: spin::MutexGuard<'_, PipeInner> = self.inner.lock();
        if self.is_write {
            let mut revents: c_short = 0;
            if inner.readers == 0 {
                revents |= POLLERR;
            }
            if inner.readers == 0 || inner.buf.len() < PIPE_CAPACITY {
                revents |= events & WRITE_EVENTS;
            }
            revents
        } else {
            let mut revents: c_short = 0;
            if !inner.buf.is_empty() || inner.writers == 0 {
                revents |= events & READ_EVENTS;
            }
            if inner.writers == 0 {
                revents |= POLLHUP;
            }
            revents
        }
    }

    /// Attempts a non-blocking read from the pipe.
    ///
    /// Returns [`PipeEndError::WrongDirection`] when invoked on the write end.
    pub fn read(&self, buf: &mut [u8]) -> Result<PipeReadOutcome, PipeEndError> {
        if self.is_write {
            return Err(PipeEndError::WrongDirection);
        }
        Ok(self.inner.lock().read(buf))
    }

    /// Non-destructively copies bytes from the pipe into `buf`.
    ///
    /// Returns [`PipeEndError::WrongDirection`] when invoked on the write end.
    pub fn peek(&self, buf: &mut [u8]) -> Result<PipeReadOutcome, PipeEndError> {
        if self.is_write {
            return Err(PipeEndError::WrongDirection);
        }
        Ok(self.inner.lock().peek(buf))
    }

    /// Removes up to `count` bytes from the front of the pipe buffer.
    ///
    /// Returns [`PipeEndError::WrongDirection`] when invoked on the write end.
    pub fn consume(&self, count: usize) -> Result<usize, PipeEndError> {
        if self.is_write {
            return Err(PipeEndError::WrongDirection);
        }
        Ok(self.inner.lock().consume(count))
    }

    /// Attempts a non-blocking write to the pipe.
    ///
    /// Returns [`PipeEndError::WrongDirection`] when invoked on the read end.
    pub fn write(&self, buf: &[u8]) -> Result<PipeWriteOutcome, PipeEndError> {
        if !self.is_write {
            return Err(PipeEndError::WrongDirection);
        }
        Ok(self.inner.lock().write(buf))
    }
}

impl Drop for PipeEnd {
    /// Decrements the reader or writer count of the shared [`PipeInner`].
    ///
    /// Only the count is adjusted here; waking suspended callers is the daemon's responsibility,
    /// because it owns the blocked-request queues.
    fn drop(&mut self) {
        let mut inner: spin::MutexGuard<'_, PipeInner> = self.inner.lock();
        if self.is_write {
            inner.writers = inner.writers.saturating_sub(1);
        } else {
            inner.readers = inner.readers.saturating_sub(1);
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
