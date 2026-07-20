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
    collections::VecDeque,
    sync::Arc,
};
use ::core::sync::atomic::{
    AtomicU64,
    Ordering,
};
use ::spin::Mutex;

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
    /// Non-blocking read from the buffer.
    ///
    /// Copies as many bytes as are available into `buf` (up to `buf.len()`). Returns
    /// [`PipeReadOutcome::WouldBlock`] when the buffer is empty but at least one writer is still
    /// open, and [`PipeReadOutcome::Eof`] when the buffer is empty and no writers remain. A
    /// zero-length read transfers no data and never blocks — POSIX `read(2)` with `count == 0`
    /// returns `0` immediately.
    fn read(&mut self, buf: &mut [u8]) -> PipeReadOutcome {
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
        // `n <= self.buf.len()`, so the drain yields exactly `n` bytes in front-to-back order.
        for (dst, src) in buf.iter_mut().zip(self.buf.drain(..n)) {
            *dst = src;
        }
        PipeReadOutcome::Read(n)
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
}

impl PipeEnd {
    /// Allocates a new pipe and returns its `(read_end, write_end)` pair.
    ///
    /// The shared [`PipeInner`] starts with `readers == 1` and `writers == 1`.
    pub fn new_pair() -> (PipeEnd, PipeEnd) {
        let pipe_id: u64 = alloc_pipe_id();
        let inner: Arc<Mutex<PipeInner>> = Arc::new(Mutex::new(PipeInner {
            buf: VecDeque::new(),
            readers: 1,
            writers: 1,
        }));
        let read_end: PipeEnd = PipeEnd {
            inner: inner.clone(),
            is_write: false,
            pipe_id,
        };
        let write_end: PipeEnd = PipeEnd {
            inner,
            is_write: true,
            pipe_id,
        };
        (read_end, write_end)
    }

    /// Returns the stable identity of the pipe this end belongs to.
    pub fn pipe_id(&self) -> u64 {
        self.pipe_id
    }

    /// Returns `true` if this is the write end, `false` if it is the read end.
    pub fn is_write(&self) -> bool {
        self.is_write
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
mod tests {
    use super::*;

    /// Drains all available bytes from a read end into a freshly allocated vector.
    fn read_all(end: &PipeEnd, max: usize) -> alloc::vec::Vec<u8> {
        let mut buf: alloc::vec::Vec<u8> = alloc::vec![0u8; max];
        match end.read(&mut buf).expect("read end") {
            PipeReadOutcome::Read(n) => {
                buf.truncate(n);
                buf
            },
            _ => alloc::vec::Vec::new(),
        }
    }

    /// Tests a basic write-then-read round-trip preserving byte order.
    #[test]
    fn round_trip_preserves_bytes() {
        let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        match w.write(&data).expect("write end") {
            PipeWriteOutcome::Wrote(n) => assert_eq!(n, 5, "should write all 5 bytes"),
            _ => panic!("write should succeed"),
        }
        assert_eq!(read_all(&r, 16), data, "read bytes should match written bytes");
    }

    /// Tests that the two ends report the same, unique pipe identity.
    #[test]
    fn ends_share_identity_unique_across_pipes() {
        let (r1, w1): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
        let (r2, _w2): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
        assert_eq!(r1.pipe_id(), w1.pipe_id(), "both ends share one identity");
        assert_ne!(r1.pipe_id(), r2.pipe_id(), "distinct pipes have distinct identities");
        assert!(!r1.is_write(), "read end must not be the write end");
        assert!(w1.is_write(), "write end must be the write end");
    }

    /// Tests that a read on an empty pipe with an open writer would block.
    #[test]
    fn empty_with_writer_would_block() {
        let (r, _w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
        let mut buf: [u8; 4] = [0; 4];
        assert!(
            matches!(r.read(&mut buf).expect("read end"), PipeReadOutcome::WouldBlock),
            "empty pipe with a writer should block"
        );
    }

    /// Tests that a read on an empty pipe with no writers returns EOF.
    #[test]
    fn empty_without_writer_is_eof() {
        let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
        drop(w);
        let mut buf: [u8; 4] = [0; 4];
        assert!(
            matches!(r.read(&mut buf).expect("read end"), PipeReadOutcome::Eof),
            "empty pipe with no writers should report EOF"
        );
    }

    /// Tests that buffered data is still readable after all writers close (drain-then-EOF).
    #[test]
    fn buffered_data_readable_after_writer_close() {
        let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
        w.write(&[9, 8, 7]).expect("write end");
        drop(w);
        assert_eq!(
            read_all(&r, 8),
            alloc::vec![9, 8, 7],
            "buffered bytes must survive writer close"
        );
        let mut buf: [u8; 4] = [0; 4];
        assert!(
            matches!(r.read(&mut buf).expect("read end"), PipeReadOutcome::Eof),
            "after draining, an empty pipe with no writers reports EOF"
        );
    }

    /// Tests that a write to a pipe whose readers have all closed is a broken pipe.
    #[test]
    fn write_without_reader_is_broken_pipe() {
        let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
        drop(r);
        assert!(
            matches!(w.write(&[1, 2, 3]).expect("write end"), PipeWriteOutcome::BrokenPipe),
            "writing with no readers should report a broken pipe"
        );
    }

    /// Tests `PIPE_BUF`-sized atomic writes: a write that does not fit entirely appends nothing.
    #[test]
    fn atomic_write_is_all_or_nothing() {
        let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
        // Fill the buffer to within less than PIPE_BUF of capacity.
        let bulk: alloc::vec::Vec<u8> = alloc::vec![0u8; PIPE_CAPACITY - 16];
        match w.write(&bulk).expect("write end") {
            PipeWriteOutcome::Wrote(n) => assert_eq!(n, PIPE_CAPACITY - 16, "bulk fill should fit"),
            _ => panic!("bulk fill should succeed"),
        }
        // A PIPE_BUF-sized write cannot fit in the remaining 16 bytes, so nothing is written.
        let atomic: [u8; PIPE_BUF] = [7u8; PIPE_BUF];
        assert!(
            matches!(w.write(&atomic).expect("write end"), PipeWriteOutcome::WouldBlock),
            "an atomic write that does not fit must append nothing"
        );
        // Free space, then the same write succeeds atomically.
        let _ = r.read(&mut [0u8; 4096]).expect("read end");
        assert!(
            matches!(w.write(&atomic).expect("write end"), PipeWriteOutcome::Wrote(PIPE_BUF)),
            "the atomic write should succeed once space is available"
        );
    }

    /// Tests that a write larger than `PIPE_BUF` may be partially accepted up to capacity.
    #[test]
    fn large_write_is_partial() {
        let (_r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
        let big: alloc::vec::Vec<u8> = alloc::vec![5u8; PIPE_CAPACITY + 4096];
        match w.write(&big).expect("write end") {
            PipeWriteOutcome::Wrote(n) => {
                assert_eq!(n, PIPE_CAPACITY, "a large write fills exactly to capacity")
            },
            _ => panic!("a large write into an empty pipe should make progress"),
        }
        // The pipe is now full: a further large write would block.
        assert!(
            matches!(w.write(&big).expect("write end"), PipeWriteOutcome::WouldBlock),
            "a full pipe should block further writes"
        );
    }

    /// Tests that using an end in the wrong direction is rejected.
    #[test]
    fn wrong_direction_is_rejected() {
        let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
        assert_eq!(
            w.read(&mut [0u8; 4]).expect_err("read on write end"),
            PipeEndError::WrongDirection,
            "reading the write end must be rejected"
        );
        assert_eq!(
            r.write(&[0u8; 4]).expect_err("write on read end"),
            PipeEndError::WrongDirection,
            "writing the read end must be rejected"
        );
    }

    /// Tests ring-buffer wrap-around across many interleaved write/read cycles.
    #[test]
    fn ring_buffer_wraps_around() {
        let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
        let chunk: [u8; 1000] = [3u8; 1000];
        // Many cycles push total bytes well past PIPE_CAPACITY, exercising wrap-around.
        for _ in 0..200 {
            assert!(
                matches!(w.write(&chunk).expect("write end"), PipeWriteOutcome::Wrote(1000)),
                "each 1000-byte chunk should fit after draining"
            );
            assert_eq!(read_all(&r, 1000).len(), 1000, "each cycle should read back 1000 bytes");
        }
    }

    /// Tests that a zero-length read returns `Read(0)` immediately and never blocks.
    #[test]
    fn zero_length_read_does_not_block() {
        let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
        // Empty pipe with an open writer would normally block, but a zero-length read must not.
        assert!(
            matches!(r.read(&mut []).expect("read end"), PipeReadOutcome::Read(0)),
            "a zero-length read on an empty pipe with a writer must return Read(0)"
        );
        // The same holds once all writers have closed: still `Read(0)`, never `Eof`.
        drop(w);
        assert!(
            matches!(r.read(&mut []).expect("read end"), PipeReadOutcome::Read(0)),
            "a zero-length read on an empty pipe with no writers must return Read(0)"
        );
    }
}
