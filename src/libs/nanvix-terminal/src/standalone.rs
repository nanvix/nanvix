// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Standalone deployment mode implementation for the terminal.
//!
//! In standalone mode, the terminal directly drives a User VM instance via `StandaloneVmHandle`,
//! bypassing the sandbox cache, gateway sockets, and control-plane infrastructure. Guest I/O is
//! bridged through IKC channels: host stdin is forwarded to the guest's stdin, and guest stdout
//! is forwarded to the host's stdout.
//!
//! Input and output are handled by independent async tasks so that a slow stdout write can never
//! stall stdin forwarding.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    error,
    info,
    warn,
};
use ::nanvix_sandbox_config::StandaloneConfig;
use ::std::io::{
    ErrorKind,
    Read,
};
use ::tokio::{
    io::{
        self,
        AsyncWriteExt,
        Stdout,
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
        // TODO (#1706): Flush conditionally based on tty semantics instead of every chunk.
        let mut stdout: Stdout = io::stdout();
        let mut io_error: Option<::std::io::Error> = None;
        while let Some(data) = output_rx.recv().await {
            if let Err(e) = stdout.write_all(&data).await {
                if e.kind() != ErrorKind::BrokenPipe {
                    error!("failed to write to stdout: {e}");
                    io_error = Some(e);
                }
                break;
            }
            if let Err(e) = stdout.flush().await {
                if e.kind() != ErrorKind::BrokenPipe {
                    error!("failed to flush stdout: {e}");
                    io_error = Some(e);
                }
                break;
            }
        }

        // Abort the input task immediately — the stdin thread blocks on read() and cannot be
        // interrupted portably, so waiting provides no benefit.
        input_handle.abort();

        match io_error {
            Some(e) => Err(e.into()),
            None => Ok(()),
        }
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
