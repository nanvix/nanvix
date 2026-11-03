// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::{
    pin::Pin,
    sync::{
        Arc,
        Mutex,
        MutexGuard,
    },
    task::{
        Context,
        Poll,
    },
    time::Duration,
};
use ::tokio::{
    sync::Notify,
    task::JoinHandle,
    time,
};

/// Maximum time that a test waits for output to be flushed.
const FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

/// Writes and flushes observed by a [`RecordingWriter`].
#[derive(Default)]
struct Recording {
    /// Data of each write, in order.
    writes: Vec<Vec<u8>>,
    /// Number of bytes written before each flush, in order.
    flushes: Vec<usize>,
}

/// Writer that records the writes and flushes it receives.
///
/// Clones share the same recording, so a test can hand a clone to the code under test and
/// inspect what happened through the original.
#[derive(Clone, Default)]
struct RecordingWriter {
    /// Writes and flushes observed so far.
    recording: Arc<Mutex<Recording>>,
    /// Notified on every flush.
    flushed: Arc<Notify>,
    /// Kind of the error that every write fails with, if any.
    write_error: Option<ErrorKind>,
}

impl RecordingWriter {
    /// Creates a writer whose writes fail with an error of the given kind.
    fn failing(kind: ErrorKind) -> Self {
        Self {
            write_error: Some(kind),
            ..Self::default()
        }
    }

    /// Locks the recording.
    fn recording(&self) -> MutexGuard<'_, Recording> {
        self.recording
            .lock()
            .expect("recording lock should not be poisoned")
    }

    /// Returns the data of each write received so far.
    fn writes(&self) -> Vec<Vec<u8>> {
        self.recording().writes.clone()
    }

    /// Returns all data written so far.
    fn output(&self) -> Vec<u8> {
        self.recording().writes.concat()
    }

    /// Returns how many bytes had been written at each flush, skipping flushes that had no new
    /// data to flush.
    fn flush_points(&self) -> Vec<usize> {
        let mut points: Vec<usize> = self.recording().flushes.clone();
        points.dedup();
        points
    }

    /// Returns how many bytes had been written at the last flush.
    fn flushed_len(&self) -> usize {
        self.recording().flushes.last().copied().unwrap_or(0)
    }

    /// Waits until at least `len` bytes have been flushed, failing the test on timeout.
    async fn wait_flushed(&self, len: usize) {
        time::timeout(FLUSH_TIMEOUT, async {
            while self.flushed_len() < len {
                self.flushed.notified().await;
            }
        })
        .await
        .expect("output should be flushed in time");
    }
}

impl AsyncWrite for RecordingWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<::std::io::Result<usize>> {
        if let Some(kind) = self.write_error {
            return Poll::Ready(Err(kind.into()));
        }
        self.recording().writes.push(buf.to_vec());
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<::std::io::Result<()>> {
        let mut recording: MutexGuard<'_, Recording> = self.recording();
        let written: usize = recording.writes.iter().map(Vec::len).sum();
        recording.flushes.push(written);
        self.flushed.notify_one();
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<::std::io::Result<()>> {
        self.poll_flush(cx)
    }
}

/// Creates a closed output channel that delivers `chunks`, in order.
fn queued_output<C: AsRef<[u8]>>(chunks: &[C]) -> mpsc::Receiver<Vec<u8>> {
    let (output_tx, output_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
        mpsc::channel(chunks.len().max(1));
    for chunk in chunks {
        output_tx
            .try_send(chunk.as_ref().to_vec())
            .expect("chunk should fit in the channel");
    }
    output_rx
}

/// Terminals flush after chunks that contain a newline, while pipes and files never do.
#[test]
fn flush_policy_flushes_after_newlines_on_terminals_only() {
    for chunk in ["line\n", "one\ntwo", "\n"] {
        assert!(FlushPolicy::Line.flushes_after(chunk.as_bytes()), "chunk={chunk:?}");
        assert!(!FlushPolicy::Block.flushes_after(chunk.as_bytes()), "chunk={chunk:?}");
    }
    for chunk in ["", "prompt> ", "progress\r"] {
        assert!(!FlushPolicy::Line.flushes_after(chunk.as_bytes()), "chunk={chunk:?}");
        assert!(!FlushPolicy::Block.flushes_after(chunk.as_bytes()), "chunk={chunk:?}");
    }
}

/// Block buffering coalesces queued chunks into a single write, flushed once none is left.
#[tokio::test]
async fn block_policy_coalesces_queued_output() {
    let mut output_rx: mpsc::Receiver<Vec<u8>> = queued_output(&["one\n", "two\n", "three"]);
    let writer: RecordingWriter = RecordingWriter::default();

    Terminal::forward_output(&mut output_rx, writer.clone(), FlushPolicy::Block)
        .await
        .expect("output should be forwarded");

    assert_eq!(writer.writes(), vec![b"one\ntwo\nthree".to_vec()]);
    assert_eq!(writer.flush_points(), vec![13]);
}

/// Line buffering flushes after every chunk that contains a newline.
#[tokio::test]
async fn line_policy_flushes_after_newlines() {
    let mut output_rx: mpsc::Receiver<Vec<u8>> =
        queued_output(&["one", " two\n", "three", " four\n", "five"]);
    let writer: RecordingWriter = RecordingWriter::default();

    Terminal::forward_output(&mut output_rx, writer.clone(), FlushPolicy::Line)
        .await
        .expect("output should be forwarded");

    assert_eq!(
        writer.writes(),
        vec![
            b"one two\n".to_vec(),
            b"three four\n".to_vec(),
            b"five".to_vec()
        ]
    );
    assert_eq!(writer.flush_points(), vec![8, 19, 23]);
}

/// Output is flushed as soon as nothing else is queued, even if the stream stays open, so a
/// guest that waits for input after writing a prompt or a reply is never stalled.
#[tokio::test]
async fn output_is_flushed_when_nothing_else_is_queued() {
    for policy in [FlushPolicy::Line, FlushPolicy::Block] {
        let (output_tx, mut output_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
            mpsc::channel(1);
        let writer: RecordingWriter = RecordingWriter::default();
        let forwarder: JoinHandle<::std::io::Result<()>> = ::tokio::spawn({
            let writer: RecordingWriter = writer.clone();
            async move { Terminal::forward_output(&mut output_rx, writer, policy).await }
        });

        output_tx
            .send(b"prompt> ".to_vec())
            .await
            .expect("prompt should be sent");
        writer.wait_flushed(8).await;
        assert_eq!(writer.output(), b"prompt> ", "policy={policy:?}");

        output_tx
            .send(b"reply".to_vec())
            .await
            .expect("reply should be sent");
        writer.wait_flushed(13).await;
        assert_eq!(writer.output(), b"prompt> reply", "policy={policy:?}");

        drop(output_tx);
        forwarder
            .await
            .expect("forwarder should not panic")
            .expect("output should be forwarded");
    }
}

/// Output that overflows the buffer is forwarded intact and in order.
#[tokio::test]
async fn output_larger_than_buffer_is_forwarded_intact() {
    let chunks: Vec<Vec<u8>> = [
        1,
        STDOUT_BUFFER_SIZE - 1,
        STDOUT_BUFFER_SIZE,
        2 * STDOUT_BUFFER_SIZE + 1,
        3,
    ]
    .into_iter()
    .zip(b'a'..)
    .map(|(len, byte)| vec![byte; len])
    .collect();
    let mut output_rx: mpsc::Receiver<Vec<u8>> = queued_output(&chunks);
    let writer: RecordingWriter = RecordingWriter::default();

    Terminal::forward_output(&mut output_rx, writer.clone(), FlushPolicy::Block)
        .await
        .expect("output should be forwarded");

    let expected: Vec<u8> = chunks.concat();
    assert_eq!(writer.output(), expected);
    assert_eq!(writer.flush_points(), vec![expected.len()]);
}

/// Write errors are returned with their kind intact, so the caller can tell a closed pipe
/// apart from other failures.
#[tokio::test]
async fn write_errors_are_returned() {
    for policy in [FlushPolicy::Line, FlushPolicy::Block] {
        for kind in [ErrorKind::BrokenPipe, ErrorKind::PermissionDenied] {
            let mut output_rx: mpsc::Receiver<Vec<u8>> = queued_output(&["lost\n"]);

            let error: ::std::io::Error =
                Terminal::forward_output(&mut output_rx, RecordingWriter::failing(kind), policy)
                    .await
                    .expect_err("forwarding should fail");

            assert_eq!(error.kind(), kind, "policy={policy:?}");
        }
    }
}
