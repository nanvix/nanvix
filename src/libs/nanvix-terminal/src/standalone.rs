// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Standalone terminal implementation.
//!
//! In standalone mode, the terminal directly drives a User VM instance via `StandaloneVmHandle`,
//! bypassing the sandbox cache, gateway sockets, and control-plane infrastructure. Guest I/O is
//! bridged through IKC channels: host stdin is forwarded to the guest's stdin, and guest stdout
//! is forwarded to the host's stdout.
//!
//! Input and output are handled by independent async tasks so that a slow stdout write can never
//! stall stdin forwarding.
//!
//! Guest output is buffered according to the tty semantics of host stdout: it is line-buffered
//! when host stdout is a terminal, and block-buffered when it is piped or redirected to a file.
//! Either way, buffered output is flushed as soon as no further guest output is pending, so output
//! that the guest is waiting on is never held back.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    debug,
    error,
    info,
    warn,
};
use ::nanvix_sandbox_config::StandaloneConfig;
use ::std::io::{
    ErrorKind,
    IsTerminal,
    Read,
};
use ::tokio::{
    io::{
        self,
        AsyncWrite,
        AsyncWriteExt,
        BufWriter,
    },
    sync::mpsc,
};
use ::uservm::standalone::{
    StandaloneVmHandle,
    StandaloneVmIo,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Size of the buffer used by the blocking stdin reader thread.
const IO_BUFFER_SIZE: usize = 4096;

/// Capacity of the bounded channel between the stdin reader thread and the async input task.
const STDIN_CHANNEL_CAPACITY: usize = 4096;

/// Capacity of the buffer that coalesces guest output before it is written to host stdout. It is
/// large enough to absorb bursts of small guest writes, so that block-buffered output reaches host
/// stdout in a few large writes rather than in one write per guest write.
const STDOUT_BUFFER_SIZE: usize = 64 * 1024;

//==================================================================================================
// Host Raw Mode
//==================================================================================================

/// Guard that places the host terminal in raw input mode while the guest runs.
///
/// In standalone mode the guest's vfsd console backend owns the terminal line discipline (canonical
/// editing, echo, special characters). For that to be authoritative, the host terminal must stop
/// cooking input: canonical mode, echo, signal generation, and input CR/NL translation are disabled
/// so keystrokes reach the guest verbatim. Output post-processing (e.g. `ONLCR`) is intentionally
/// left enabled, because guest *program* output bypasses the guest line discipline and still relies
/// on the host to render newlines.
///
/// The switch is a no-op unless host stdin is a real terminal, so piped or redirected input (tests,
/// scripts, HTTP mode) is forwarded byte-for-byte and unaffected. The previous attributes are
/// restored when the guard drops, i.e. when the guest exits.
struct RawModeGuard {
    /// The terminal attributes to restore on drop; `None` when no switch was performed.
    #[cfg(unix)]
    saved: Option<::libc::termios>,
}

impl RawModeGuard {
    /// Switches host stdin to raw input mode when it is a terminal, returning a restoring guard.
    #[cfg(unix)]
    fn new() -> Self {
        // Only switch a real terminal; leave piped/redirected stdin untouched.
        // SAFETY: `isatty` only inspects the descriptor and is always safe to call.
        if unsafe { ::libc::isatty(::libc::STDIN_FILENO) } != 1 {
            return Self { saved: None };
        }
        // SAFETY: `termios` is a plain C struct; a zeroed value is a valid initial state that
        // `tcgetattr` fully overwrites.
        let mut termios: ::libc::termios = unsafe { ::std::mem::zeroed() };
        // SAFETY: `termios` points to a valid, owned structure for the duration of the call.
        if unsafe { ::libc::tcgetattr(::libc::STDIN_FILENO, &mut termios) } != 0 {
            warn!("standalone: failed to read host terminal attributes; leaving cooked mode");
            return Self { saved: None };
        }
        let saved: ::libc::termios = termios;
        Self::make_raw_input(&mut termios);
        // SAFETY: `termios` points to a valid, owned structure for the duration of the call.
        if unsafe { ::libc::tcsetattr(::libc::STDIN_FILENO, ::libc::TCSANOW, &termios) } != 0 {
            warn!("standalone: failed to set host terminal to raw mode; leaving cooked mode");
            return Self { saved: None };
        }
        info!("standalone: host terminal switched to raw input mode");
        Self { saved: Some(saved) }
    }

    /// Non-Unix hosts do not expose POSIX termios; the switch is a no-op there.
    #[cfg(not(unix))]
    fn new() -> Self {
        Self {}
    }

    /// Clears the input-cooking, echo, and signal flags, leaving output post-processing intact.
    #[cfg(unix)]
    fn make_raw_input(termios: &mut ::libc::termios) {
        // Disable canonical mode, echo, signal generation (`^C`/`^Z`), and extended input
        // processing so the guest line discipline is authoritative for editing, echo, and special
        // characters.
        termios.c_lflag &= !(::libc::ICANON
            | ::libc::ECHO
            | ::libc::ECHOE
            | ::libc::ECHOK
            | ::libc::ECHONL
            | ::libc::ISIG
            | ::libc::IEXTEN);
        // Disable input CR/NL translation, flow control, and break handling so bytes reach the
        // guest verbatim; the guest maps CR to NL itself (`ICRNL` in its own termios).
        termios.c_iflag &= !(::libc::ICRNL
            | ::libc::INLCR
            | ::libc::IGNCR
            | ::libc::IXON
            | ::libc::IXOFF
            | ::libc::BRKINT
            | ::libc::ISTRIP
            | ::libc::INPCK);
        // Deliver each byte as soon as it is typed.
        termios.c_cc[::libc::VMIN] = 1;
        termios.c_cc[::libc::VTIME] = 0;
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(saved) = self.saved {
            // SAFETY: `saved` is a valid termios previously read from this same terminal.
            unsafe {
                ::libc::tcsetattr(::libc::STDIN_FILENO, ::libc::TCSANOW, &saved);
            }
        }
    }
}

//==================================================================================================
// Output Buffering
//==================================================================================================

/// Policy that decides when buffered guest output is flushed to host stdout.
///
/// The policy mirrors how C stdio buffers standard output: line buffering when the stream is a
/// terminal, so that each line is displayed as soon as it is complete, and block buffering
/// otherwise, so that piped or redirected output is written out in a few large writes instead of
/// one write per guest write.
///
/// Regardless of the policy, buffered output is also flushed whenever no further guest output is
/// pending (see `Terminal::forward_output()`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FlushPolicy {
    /// Flush whenever a chunk of output contains a newline (host stdout is a terminal).
    Line,
    /// Flush only when the buffer fills up (host stdout is a pipe or a file).
    Block,
}

impl FlushPolicy {
    /// Selects the policy that matches the tty semantics of host stdout.
    fn for_host_stdout() -> Self {
        if ::std::io::stdout().is_terminal() {
            Self::Line
        } else {
            Self::Block
        }
    }

    /// Checks whether buffered output must be flushed right after `chunk` is buffered.
    fn flushes_after(self, chunk: &[u8]) -> bool {
        match self {
            Self::Line => chunk.contains(&b'\n'),
            Self::Block => false,
        }
    }
}

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Terminal interface for interacting with user VMs in standalone mode.
///
/// Spawns a User VM via `StandaloneVmHandle` and bridges host stdin/stdout with the guest's
/// I/O channels.
///
pub struct Terminal {
    /// Configuration for launching new VMs.
    config: StandaloneConfig,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Terminal {
    ///
    /// # Description
    ///
    /// Creates a new Terminal instance.
    ///
    /// # Parameters
    ///
    /// - `config`: Configuration for launching VMs.
    ///
    pub fn new(config: StandaloneConfig) -> Self {
        Self { config }
    }

    ///
    /// # Description
    ///
    /// Runs the terminal by spawning a User VM and bridging host stdin/stdout with guest I/O.
    ///
    /// # Parameters
    ///
    /// - `_tenant_id`: Unused in standalone mode.
    /// - `_app_name`: Unused in standalone mode.
    /// - `guest_binary_path`: Path to the guest binary to execute.
    /// - `guest_binary_args`: Arguments to pass to the guest binary.
    ///
    /// # Returns
    ///
    /// On success, returns the exit code of the guest program. On failure, returns an error
    /// describing what went wrong.
    ///
    pub async fn run(
        &mut self,
        _tenant_id: Option<&str>,
        _app_name: Option<&str>,
        guest_binary_path: &str,
        guest_binary_args: &str,
    ) -> Result<i32> {
        info!("spawning VM in standalone terminal mode");

        // Put the host terminal in raw mode for the lifetime of the guest so its in-vfsd line
        // discipline is authoritative for canonical editing, echo, and special characters. The
        // guard restores the previous attributes on drop (guest exit). It is a no-op unless host
        // stdin is a terminal, so piped/redirected input (tests, scripts) is forwarded verbatim.
        let _raw_mode: RawModeGuard = RawModeGuard::new();

        let initrd_args: Option<String> = if guest_binary_args.is_empty() {
            None
        } else {
            Some(guest_binary_args.to_string())
        };

        let (handle, io): (StandaloneVmHandle, StandaloneVmIo) = StandaloneVmHandle::spawn(
            self.config.kernel_binary_path().to_string(),
            Some(guest_binary_path.to_string()),
            initrd_args,
            self.config.kernel_args().map(|s| s.to_string()),
            self.config.ramfs_filename().map(|s| s.to_string()),
            self.config.console_file().map(|s| s.to_string()),
            self.config.snapshot_path().map(|s| s.to_string()),
            self.config.mount_directory().map(|s| s.to_string()),
            self.config.networking_mode(),
            self.config.host_filter(),
            #[cfg(feature = "gdb")]
            self.config.gdb_port(),
        );

        // Bridge host stdin/stdout with guest I/O channels.
        let io_result: Result<()> = Self::bridge_io(io).await;
        if let Err(ref e) = io_result {
            warn!("terminal I/O bridge ended with error: {e:?}");
        }

        match handle.wait().await {
            Ok(exit_status) => {
                info!("VM exited (exit_status={exit_status})");
                Ok(i32::from(exit_status))
            },
            Err(error) => {
                error!("VM failed (error={error:?})");
                Ok(-1)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Bridges host stdin/stdout with the guest's I/O channels.
    ///
    /// Input and output are handled by independent async tasks: a dedicated task forwards host
    /// stdin to the guest input channel, while the current task drains guest output to host
    /// stdout. This decoupling ensures that a slow stdout write (e.g., a `spawn_blocking` call
    /// on Windows piped stdout) can never stall stdin forwarding.
    ///
    /// Guest output is buffered according to the tty semantics of host stdout (see
    /// `FlushPolicy`).
    ///
    /// EOF propagation is structural: when the stdin reader thread exits (EOF, error, or receiver
    /// dropped), `stdin_tx` is dropped, causing `stdin_rx.recv()` to return `None` in the input
    /// task. The input task then exits and drops `input_tx`, which signals EOF to the guest via
    /// the standalone I/O handler.
    ///
    async fn bridge_io(io: StandaloneVmIo) -> Result<()> {
        let StandaloneVmIo {
            mut output_rx,
            input_tx,
        } = io;

        // --- Input path (independent task): host stdin → guest input ---
        let input_handle: ::tokio::task::JoinHandle<()> = ::tokio::spawn(async move {
            let (stdin_tx, mut stdin_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
                mpsc::channel(STDIN_CHANNEL_CAPACITY);

            // Spawn a dedicated OS thread for blocking stdin reads. Not joined — the
            // thread may block on stdin.read() until the process exits; this is acceptable
            // during shutdown.
            let _stdin_handle: ::std::thread::JoinHandle<()> = ::std::thread::spawn(move || {
                Self::stdin_thread(stdin_tx);
            });

            // Forward all stdin data to the guest. When stdin_rx returns None the stdin thread
            // has exited (EOF or error). Dropping input_tx signals EOF to the guest.
            while let Some(data) = stdin_rx.recv().await {
                if input_tx.send(data).await.is_err() {
                    break;
                }
            }
            // input_tx dropped here → standalone_io_handler sees channel close → guest EOF.
        });

        // --- Output path (current task): guest output → host stdout ---
        let policy: FlushPolicy = FlushPolicy::for_host_stdout();
        debug!("standalone: forwarding guest output to host stdout (policy={policy:?})");
        let output_result: ::std::io::Result<()> =
            Self::forward_output(&mut output_rx, io::stdout(), policy).await;

        // Abort the input task immediately — the stdin thread blocks on read() and cannot be
        // interrupted portably, so waiting provides no benefit.
        input_handle.abort();

        match output_result {
            Ok(()) => Ok(()),
            // The reader of host stdout went away (e.g., the other end of a pipe was closed), so
            // there is nobody left to forward guest output to.
            Err(e) if e.kind() == ErrorKind::BrokenPipe => Ok(()),
            Err(e) => {
                error!("failed to forward guest output to stdout: {e}");
                Err(e.into())
            },
        }
    }

    ///
    /// # Description
    ///
    /// Forwards guest output to `writer` until the output stream ends.
    ///
    /// Output is staged in a buffer that is flushed according to `policy`. On top of that, the
    /// buffer is flushed whenever no further guest output is pending. Output that the guest is
    /// waiting on, such as a prompt, an echoed keystroke, or a reply to a peer that drives the
    /// guest through pipes, is thus never held back: buffering only coalesces output that the
    /// guest produces faster than it can be written out.
    ///
    /// # Parameters
    ///
    /// - `output_rx`: Channel that delivers the output written by the guest.
    /// - `writer`: Writer to which guest output is forwarded.
    /// - `policy`: Policy that decides when buffered output is flushed.
    ///
    /// # Returns
    ///
    /// On success, returns empty. On failure, returns the error reported by `writer`.
    ///
    async fn forward_output<W>(
        output_rx: &mut mpsc::Receiver<Vec<u8>>,
        writer: W,
        policy: FlushPolicy,
    ) -> ::std::io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        let mut writer: BufWriter<W> = BufWriter::with_capacity(STDOUT_BUFFER_SIZE, writer);
        while let Some(data) = output_rx.recv().await {
            writer.write_all(&data).await?;
            if policy.flushes_after(&data) || output_rx.is_empty() {
                writer.flush().await?;
            }
        }
        // Flush once more as the stream ends: `BufWriter` discards, rather than flushes, any
        // output still buffered when it is dropped.
        writer.flush().await
    }

    ///
    /// # Description
    ///
    /// Thread function for reading from stdin in a blocking manner.
    ///
    /// Reads chunks from host stdin and forwards them to the async input task via `stdin_tx`.
    /// When EOF is reached, the read returns a non-recoverable error, or the receiver is dropped,
    /// the thread exits and `stdin_tx` is dropped — which propagates EOF structurally through the
    /// channel chain. Transient `EINTR` errors are retried automatically.
    ///
    /// # Parameters
    ///
    /// - `stdin_tx`: Bounded channel sender to forward stdin data to the async input task.
    ///
    fn stdin_thread(stdin_tx: mpsc::Sender<Vec<u8>>) {
        let mut stdin: ::std::io::Stdin = ::std::io::stdin();
        let mut buffer: [u8; IO_BUFFER_SIZE] = [0; IO_BUFFER_SIZE];

        loop {
            match stdin.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    if stdin_tx.blocking_send(buffer[..n].to_vec()).is_err() {
                        break;
                    }
                },
                Err(error) => {
                    if error.kind() == ::std::io::ErrorKind::Interrupted {
                        // Retry on interrupts.
                        continue;
                    }
                    error!("failed to read from stdin: {error}");
                    break;
                },
            }
        }
        // stdin_tx dropped here → stdin_rx.recv() returns None → input_tx dropped → guest EOF.
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
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

        fn poll_shutdown(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<::std::io::Result<()>> {
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

                let error: ::std::io::Error = Terminal::forward_output(
                    &mut output_rx,
                    RecordingWriter::failing(kind),
                    policy,
                )
                .await
                .expect_err("forwarding should fail");

                assert_eq!(error.kind(), kind, "policy={policy:?}");
            }
        }
    }
}
