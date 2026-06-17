// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Description
//!
//! Decouples guest console output from the WHP partition lock.
//!
//! On Windows/WHP the vCPU run loop holds the partition lock across
//! [`Emulator::handle_pmio_access()`](crate::vmm::emulator::Emulator::handle_pmio_access), which
//! performs per-byte host writes for guest console output (port `0xE9`). If the host sink
//! back-pressures (for example a starved pipe reader under heavy host load), a blocking host write
//! while the partition lock is held freezes the guest.
//!
//! [`AsyncConsoleWriter`] breaks this coupling: [`Write::write()`] only copies bytes into a bounded
//! in-process buffer and returns immediately, while a dedicated thread drains the buffer to the
//! underlying sink off the partition lock. When the buffer is full, excess bytes are dropped and
//! counted instead of blocking the producer, so the vCPU thread never stalls on host I/O while it
//! holds the partition lock.

//==================================================================================================
// Imports
//==================================================================================================

use ::log::{
    error,
    warn,
};
use ::std::{
    collections::VecDeque,
    io::{
        self,
        Write,
    },
    sync::{
        Arc,
        Condvar,
        Mutex,
        MutexGuard,
    },
    thread::{
        self,
        JoinHandle,
    },
    time::{
        Duration,
        Instant,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of bytes buffered before new console bytes are dropped.
const BUFFER_CAPACITY_BYTES: usize = 4 * 1024 * 1024;

/// Upper bound on how long [`AsyncConsoleWriter::flush()`] waits for the drain thread.
const FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

/// Upper bound on how long [`AsyncConsoleWriter`]'s drop handler waits for the drain thread to
/// stop before detaching it. A wedged drain thread (blocked in a host write against a sink that
/// never drains) must not block VM teardown.
const DRAIN_JOIN_GRACE: Duration = Duration::from_secs(2);

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// State shared between the producer (vCPU thread) and the drain thread.
///
struct State {
    /// Console bytes awaiting write to the underlying sink.
    data: VecDeque<u8>,
    /// Total bytes accepted into `data` over the writer's lifetime.
    enqueued: u64,
    /// Total bytes written to (and flushed on) the underlying sink.
    written: u64,
    /// Number of bytes dropped because the buffer was full.
    dropped: u64,
    /// Set when the producer is shutting down; the drain thread flushes then exits.
    closing: bool,
    /// Set once the drain thread has stopped (clean exit or unrecoverable sink error).
    stopped: bool,
}

///
/// # Description
///
/// Synchronization primitives shared with the drain thread.
///
struct Channel {
    /// Shared mutable state.
    state: Mutex<State>,
    /// Signaled when bytes are enqueued or `closing` is set.
    nonempty: Condvar,
    /// Signaled when bytes are drained or the drain thread stops.
    drained: Condvar,
}

///
/// # Description
///
/// A [`Write`] adapter that hands bytes to a dedicated drain thread via a bounded buffer.
///
pub struct AsyncConsoleWriter {
    /// Shared channel between the producer and the drain thread.
    channel: Arc<Channel>,
    /// Handle to the drain thread, joined on drop.
    handle: Option<JoinHandle<()>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Channel {
    ///
    /// # Description
    ///
    /// Locks the shared state, recovering the guard if the mutex was poisoned.
    ///
    /// # Returns
    ///
    /// A guard for the shared state.
    ///
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl AsyncConsoleWriter {
    ///
    /// # Description
    ///
    /// Wraps `inner` so that console writes are decoupled from the caller's lock. A dedicated
    /// thread owns `inner` and performs the (potentially blocking) host writes.
    ///
    /// # Parameters
    ///
    /// - `inner`: Underlying console sink.
    ///
    /// # Returns
    ///
    /// A new [`AsyncConsoleWriter`].
    ///
    pub fn new(inner: Box<dyn Write + Send>) -> Self {
        let channel: Arc<Channel> = Arc::new(Channel {
            state: Mutex::new(State {
                data: VecDeque::new(),
                enqueued: 0,
                written: 0,
                dropped: 0,
                closing: false,
                stopped: false,
            }),
            nonempty: Condvar::new(),
            drained: Condvar::new(),
        });

        let drain_channel: Arc<Channel> = channel.clone();
        let handle: Option<JoinHandle<()>> = match thread::Builder::new()
            .name("nanvix-console".to_string())
            .spawn(move || drain_loop(&drain_channel, inner))
        {
            Ok(handle) => Some(handle),
            // If the drain thread could not be spawned, log the underlying I/O error and mark the
            // writer stopped so that `flush()` never blocks waiting for a thread that does not
            // exist and `write()` accounts bytes as dropped rather than buffering them forever.
            Err(e) => {
                error!("AsyncConsoleWriter::new(): failed to spawn console drain thread: {}", e);
                channel.lock().stopped = true;
                None
            },
        };

        Self { channel, handle }
    }
}

impl Write for AsyncConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut state: MutexGuard<'_, State> = self.channel.lock();

        // If the drain thread is gone, account the bytes as dropped but report success so the
        // emulator never errors out (and tears the guest down) over best-effort console output.
        if state.stopped {
            state.dropped = state.dropped.saturating_add(buf.len() as u64);
            return Ok(buf.len());
        }

        let was_empty: bool = state.data.is_empty();
        let free: usize = BUFFER_CAPACITY_BYTES.saturating_sub(state.data.len());
        let take: usize = buf.len().min(free);
        if take > 0 {
            state.data.extend(&buf[..take]);
            state.enqueued = state.enqueued.saturating_add(take as u64);
        }
        if take < buf.len() {
            state.dropped = state.dropped.saturating_add((buf.len() - take) as u64);
        }
        drop(state);

        // Wake the drain thread only on an empty -> non-empty transition: while the buffer is
        // non-empty the drain thread is guaranteed to revisit it without an explicit wakeup.
        if was_empty && take > 0 {
            self.channel.nonempty.notify_one();
        }

        // Always report the full length as accepted; dropped bytes are tracked separately.
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut state: MutexGuard<'_, State> = self.channel.lock();
        let target: u64 = state.enqueued;
        // Ensure the drain thread is awake even if a previous wakeup was elided.
        self.channel.nonempty.notify_one();
        let deadline: Instant = Instant::now() + FLUSH_TIMEOUT;
        while state.written < target && !state.stopped {
            let now: Instant = Instant::now();
            if now >= deadline {
                warn!("AsyncConsoleWriter::flush(): timed out draining console buffer");
                break;
            }
            let (guard, _timeout) = self
                .channel
                .drained
                .wait_timeout(state, deadline - now)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state = guard;
        }
        Ok(())
    }
}

impl Drop for AsyncConsoleWriter {
    fn drop(&mut self) {
        // Signal shutdown and wake the drain thread.
        self.channel.lock().closing = true;
        self.channel.nonempty.notify_one();

        // Wait briefly for the drain thread to stop on its own. If it is wedged in a blocking
        // host write against a sink that never drains, do NOT block teardown on it: leave it
        // detached so that process exit reclaims it. Joining unconditionally here would
        // reintroduce the very hang this writer exists to prevent.
        let stopped: bool = {
            let mut state: MutexGuard<'_, State> = self.channel.lock();
            let deadline: Instant = Instant::now() + DRAIN_JOIN_GRACE;
            while !state.stopped {
                let now: Instant = Instant::now();
                if now >= deadline {
                    break;
                }
                let (guard, _timeout) = self
                    .channel
                    .drained
                    .wait_timeout(state, deadline - now)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state = guard;
            }
            state.stopped
        };

        let dropped: u64 = self.channel.lock().dropped;
        if dropped > 0 {
            warn!("AsyncConsoleWriter: dropped {dropped} console byte(s) under back-pressure");
        }

        if stopped {
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        } else {
            warn!(
                "AsyncConsoleWriter: console drain thread still busy at shutdown; detaching to \
                 avoid blocking VM teardown"
            );
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Drains buffered bytes to `inner` until the writer is closing and the buffer is empty.
///
/// # Parameters
///
/// - `channel`: Shared channel between the producer and the drain thread.
/// - `inner`: Underlying console sink owned by the drain thread.
///
fn drain_loop(channel: &Arc<Channel>, mut inner: Box<dyn Write + Send>) {
    let mut scratch: Vec<u8> = Vec::new();
    loop {
        let mut state: MutexGuard<'_, State> = channel.lock();
        while state.data.is_empty() && !state.closing {
            state = channel
                .nonempty
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        if state.data.is_empty() && state.closing {
            state.stopped = true;
            channel.drained.notify_all();
            return;
        }
        scratch.clear();
        scratch.extend(state.data.drain(..));
        drop(state);

        let count: u64 = scratch.len() as u64;
        let result: io::Result<()> = inner.write_all(&scratch).and_then(|()| inner.flush());

        let mut state: MutexGuard<'_, State> = channel.lock();
        // Account the bytes as written even on error so that `flush()` waiters are released;
        // there is nothing useful to retry against a broken sink.
        state.written = state.written.saturating_add(count);
        if let Err(error) = result {
            error!("AsyncConsoleWriter: console sink write failed; stopping drain ({error})");
            state.stopped = true;
            channel.drained.notify_all();
            return;
        }
        if state.data.is_empty() {
            channel.drained.notify_all();
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

/// Unit tests for the asynchronous console writer.
#[cfg(test)]
mod tests {
    use super::{
        AsyncConsoleWriter,
        BUFFER_CAPACITY_BYTES,
        DRAIN_JOIN_GRACE,
        FLUSH_TIMEOUT,
    };
    use ::std::{
        io::{
            self,
            Write,
        },
        sync::{
            Arc,
            Condvar,
            Mutex,
            MutexGuard,
        },
        time::{
            Duration,
            Instant,
        },
    };

    ///
    /// # Description
    ///
    /// A gate that a [`TestSink`] waits on before each write completes. It lets a test hold the
    /// drain thread inside the sink to deterministically model a back-pressured host sink.
    ///
    struct Gate {
        /// Whether the gate is open (writes may proceed).
        open: Mutex<bool>,
        /// Signaled when the gate is opened.
        cvar: Condvar,
    }

    impl Gate {
        /// Creates a gate in the given initial state.
        fn new(open: bool) -> Arc<Self> {
            Arc::new(Self {
                open: Mutex::new(open),
                cvar: Condvar::new(),
            })
        }

        /// Opens the gate and wakes any waiter.
        fn open(&self) {
            *self.open.lock().expect("gate lock") = true;
            self.cvar.notify_all();
        }

        /// Blocks until the gate is open.
        fn wait(&self) {
            let mut open: MutexGuard<'_, bool> = self.open.lock().expect("gate lock");
            while !*open {
                open = self.cvar.wait(open).expect("gate wait");
            }
        }
    }

    ///
    /// # Description
    ///
    /// A [`Write`] sink for tests. It records everything written and can optionally block on a
    /// [`Gate`] and/or fail, to exercise back-pressure and error handling.
    ///
    struct TestSink {
        /// Bytes recorded by the sink.
        recorded: Arc<Mutex<Vec<u8>>>,
        /// Optional gate the sink waits on before each write.
        gate: Option<Arc<Gate>>,
        /// When true, every write fails.
        fail: bool,
    }

    impl Write for TestSink {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if let Some(gate) = &self.gate {
                gate.wait();
            }
            if self.fail {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "sink failed"));
            }
            self.recorded
                .lock()
                .expect("recorded lock")
                .extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Builds a non-blocking, non-failing recording sink and returns its shared buffer.
    fn recording_sink() -> (Arc<Mutex<Vec<u8>>>, TestSink) {
        let recorded: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let sink: TestSink = TestSink {
            recorded: recorded.clone(),
            gate: None,
            fail: false,
        };
        (recorded, sink)
    }

    /// Writing then flushing delivers every byte, in order, to the underlying sink.
    #[test]
    fn write_then_flush_delivers_all_bytes_in_order() {
        let (recorded, sink) = recording_sink();
        let mut writer: AsyncConsoleWriter = AsyncConsoleWriter::new(Box::new(sink));

        let payload: &[u8] = b"the quick brown fox jumps over the lazy dog";
        for chunk in payload.chunks(3) {
            writer.write_all(chunk).expect("write");
        }
        writer.flush().expect("flush");

        assert_eq!(recorded.lock().expect("recorded lock").as_slice(), payload);
    }

    /// `flush()` is a barrier: after it returns, all previously written bytes are durable.
    #[test]
    fn flush_is_a_barrier_for_prior_writes() {
        let (recorded, sink) = recording_sink();
        let mut writer: AsyncConsoleWriter = AsyncConsoleWriter::new(Box::new(sink));

        writer.write_all(b"first;").expect("write");
        writer.flush().expect("flush");
        assert_eq!(recorded.lock().expect("recorded lock").as_slice(), b"first;");

        writer.write_all(b"second").expect("write");
        writer.flush().expect("flush");
        assert_eq!(recorded.lock().expect("recorded lock").as_slice(), b"first;second");
    }

    /// An oversized write reports the full length as accepted, drops only the excess beyond the
    /// bounded buffer, and never blocks the producer.
    #[test]
    fn oversized_write_reports_full_length_and_drops_excess() {
        // Hold the drain thread inside the sink so the bounded buffer fills deterministically.
        let gate: Arc<Gate> = Gate::new(false);
        let recorded: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let mut writer: AsyncConsoleWriter = AsyncConsoleWriter::new(Box::new(TestSink {
            recorded: recorded.clone(),
            gate: Some(gate.clone()),
            fail: false,
        }));

        let payload: Vec<u8> = vec![0x5A; BUFFER_CAPACITY_BYTES + 4096];

        // The producer reports the whole buffer as accepted even though only the bounded prefix
        // fits; the excess is dropped rather than blocking on the wedged sink.
        let accepted: usize = writer.write(&payload).expect("write");
        assert_eq!(accepted, payload.len());

        // Release the sink and confirm exactly the buffered prefix is delivered.
        gate.open();
        writer.flush().expect("flush");
        assert_eq!(recorded.lock().expect("recorded lock").len(), BUFFER_CAPACITY_BYTES);
    }

    /// After the sink fails, the drain thread stops, yet console writes keep reporting success
    /// so a broken host sink never tears down the guest.
    #[test]
    fn writes_after_sink_failure_still_report_success() {
        let mut writer: AsyncConsoleWriter = AsyncConsoleWriter::new(Box::new(TestSink {
            recorded: Arc::new(Mutex::new(Vec::new())),
            gate: None,
            fail: true,
        }));

        // Trigger the drain thread so it hits the sink error and stops.
        writer.write_all(b"trigger").expect("write");
        writer.flush().expect("flush");

        // The sink is broken, but writes must still succeed (bytes are dropped, not surfaced).
        let accepted: usize = writer
            .write(b"after failure")
            .expect("write must not error");
        assert_eq!(accepted, b"after failure".len());
    }

    /// Dropping the writer flushes any still-buffered output before the drain thread exits.
    #[test]
    fn drop_flushes_pending_output() {
        let recorded: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        {
            let mut writer: AsyncConsoleWriter = AsyncConsoleWriter::new(Box::new(TestSink {
                recorded: recorded.clone(),
                gate: None,
                fail: false,
            }));
            writer.write_all(b"pending on drop").expect("write");
            // Drop without an explicit flush: the drain thread must flush before exiting.
        }
        assert_eq!(recorded.lock().expect("recorded lock").as_slice(), b"pending on drop");
    }

    /// `flush()` is bounded: against a permanently wedged sink it returns after the timeout
    /// rather than hanging. This test takes about `FLUSH_TIMEOUT` to run by design.
    #[test]
    fn flush_returns_after_timeout_when_sink_is_wedged() {
        let gate: Arc<Gate> = Gate::new(false);
        let mut writer: AsyncConsoleWriter = AsyncConsoleWriter::new(Box::new(TestSink {
            recorded: Arc::new(Mutex::new(Vec::new())),
            gate: Some(gate.clone()),
            fail: false,
        }));

        writer.write_all(b"wedged").expect("write");

        let start: Instant = Instant::now();
        writer.flush().expect("flush");
        assert!(start.elapsed() >= FLUSH_TIMEOUT);

        // Release the gate so the drain thread can exit cleanly when the writer drops.
        gate.open();
    }

    /// Dropping the writer does not block indefinitely when the drain thread is wedged in a
    /// host write: it detaches after a bounded grace period. This takes about `DRAIN_JOIN_GRACE`.
    #[test]
    fn drop_does_not_block_when_drain_thread_is_wedged() {
        let gate: Arc<Gate> = Gate::new(false);

        let start: Instant = Instant::now();
        {
            let mut writer: AsyncConsoleWriter = AsyncConsoleWriter::new(Box::new(TestSink {
                recorded: Arc::new(Mutex::new(Vec::new())),
                gate: Some(gate.clone()),
                fail: false,
            }));
            writer.write_all(b"wedged").expect("write");
            // Drop while the drain thread is stuck in the sink: Drop must time out and detach.
        }
        let elapsed: Duration = start.elapsed();
        assert!(elapsed >= DRAIN_JOIN_GRACE);
        // Allow generous scheduling slack on top of the bounded grace period: a busy or slow CI
        // runner may deschedule this thread around the timeout boundary even when Drop is correctly
        // bounded, so the upper bound only guards against a true hang (a wedged join would block for
        // the full `FLUSH_TIMEOUT` or longer).
        const SCHEDULING_SLACK: Duration = Duration::from_secs(2);
        assert!(elapsed < DRAIN_JOIN_GRACE + FLUSH_TIMEOUT + SCHEDULING_SLACK);

        // Release the detached drain thread so it exits instead of lingering.
        gate.open();
    }
}
