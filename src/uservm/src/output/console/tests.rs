// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
