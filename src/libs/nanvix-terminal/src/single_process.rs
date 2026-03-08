// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Single-process deployment mode implementation for the terminal.
//!
//! In single-process mode, the terminal connects to a gateway socket provided by the
//! simplified sandbox cache (which embeds linuxd as an async task) and streams I/O between
//! stdin/stdout and the gateway. This mirrors the multi-process architecture but uses
//! `SimpleSandboxCache` instead of `SandboxCache`, keeping everything in a single process.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::libc::{
    c_int,
    sigaction,
    sigemptyset,
    SIGUSR1,
};
use ::log::{
    error,
    info,
    warn,
};
use ::nanvix_sandbox::{
    simple_cache::{
        SimpleSandboxCache,
        SimpleSandboxCacheConfig,
    },
    syscomm::{
        SocketStream,
        SocketStreamReader,
        SocketStreamWriter,
        SocketType,
        UnboundSocket,
        WriteAll,
    },
    UserVmIdentifier,
};
use ::std::{
    io::{
        ErrorKind,
        Read,
    },
    mem,
    ptr,
    sync::Arc,
};
use ::tokio::{
    io::{
        self,
        AsyncWriteExt,
        Stdout,
    },
    signal::unix::{
        signal,
        Signal,
        SignalKind,
    },
    sync::{
        mpsc,
        mpsc::{
            UnboundedReceiver,
            UnboundedSender,
        },
        Mutex,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default application name for terminal sessions.
const DEFAULT_APP_NAME: &str = "nanvixd-terminal";

/// Size of I/O buffers for terminal communication.
/// Set to 1 byte for character-by-character I/O to ensure responsive terminal interaction.
const IO_BUFFER_SIZE: usize = 1;

/// Signal used to interrupt blocking operations in stdin thread.
const INTERRUPT_SIGNAL: c_int = SIGUSR1;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Terminal interface for interacting with user VMs in single-process mode.
///
/// Connects to a gateway socket provided by the simplified sandbox cache (which embeds linuxd)
/// and streams I/O between stdin/stdout and the gateway.
///
/// # Type Parameters
///
/// - `T`: Custom state type for the syscall table. Must implement `Sync + Send + Clone + Default`.
///   Use `()` if no custom state is required.
///
pub struct Terminal<T> {
    /// Configuration for simplified sandbox cache management.
    config: SimpleSandboxCacheConfig<T>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<T: Sync + Send + Clone + Default + 'static> Terminal<T> {
    ///
    /// # Description
    ///
    /// Creates a new Terminal instance.
    ///
    /// # Parameters
    ///
    /// - `config`: Configuration for simplified sandbox cache management.
    ///
    pub fn new(config: SimpleSandboxCacheConfig<T>) -> Self {
        Self { config }
    }

    ///
    /// # Description
    ///
    /// Runs the terminal interface.
    ///
    /// Creates a simplified sandbox cache (with embedded linuxd), spawns a User VM, and bridges
    /// host stdin/stdout with the gateway socket.
    ///
    /// # Parameters
    ///
    /// - `tenant_id`: Optional tenant identifier. If `None`, a default tenant ID is used.
    /// - `app_name`: Optional application name. If `None`, a default application name is used.
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
        tenant_id: Option<&str>,
        app_name: Option<&str>,
        guest_binary_path: &str,
        guest_binary_args: &str,
    ) -> Result<i32> {
        info!("spawning VM in single-process terminal mode");

        let sandbox_cache: Arc<Mutex<SimpleSandboxCache<T>>> =
            SimpleSandboxCache::new(self.config.clone()).await?;
        let mut signals: Signal = signal(SignalKind::interrupt())?;

        let tenant_id: String = match tenant_id {
            Some(s) => s.to_owned(),
            None => Self::get_current_user_name()?,
        };
        let app_name: String = app_name
            .map(|s| s.to_owned())
            .unwrap_or_else(|| DEFAULT_APP_NAME.to_owned());
        let (uservm_id, gateway_sockaddr, gateway_socket_type): (
            UserVmIdentifier,
            String,
            SocketType,
        ) = sandbox_cache
            .lock()
            .await
            .get(
                &tenant_id,
                guest_binary_path,
                &app_name,
                if guest_binary_args.is_empty() {
                    None
                } else {
                    Some(guest_binary_args.to_string())
                },
            )
            .await?;

        let gateway_stream: SocketStream = UnboundSocket::new(gateway_socket_type)
            .connect(&gateway_sockaddr)
            .await?;

        // Create channel for stdin data.
        let (stdin_tx, mut stdin_rx): (UnboundedSender<Vec<u8>>, UnboundedReceiver<Vec<u8>>) =
            mpsc::unbounded_channel();

        // Create channel for EOF notification.
        let (eof_tx, mut eof_rx): (UnboundedSender<()>, UnboundedReceiver<()>) =
            mpsc::unbounded_channel();

        // Create channel for thread ID communication.
        let (thread_id_tx, mut thread_id_rx): (UnboundedSender<u64>, UnboundedReceiver<u64>) =
            mpsc::unbounded_channel();

        // Spawn a dedicated thread for blocking stdin reads.
        let _stdin_handle: ::std::thread::JoinHandle<()> = ::std::thread::spawn(move || {
            Self::stdin_thread(stdin_tx, thread_id_tx, eof_tx);
        });

        // Wait for the thread ID to be sent.
        let stdin_thread_id: u64 = thread_id_rx.recv().await.ok_or_else(|| {
            let reason: &str = "failed to receive id of stdin thread";
            error!("{reason}");
            anyhow::anyhow!(reason)
        })?;

        let mut stdout: Stdout = io::stdout();
        let mut gateway_buffer: [u8; IO_BUFFER_SIZE] = [0; IO_BUFFER_SIZE];

        let (mut gateway_stream_rx, gateway_stream_tx): (SocketStreamReader, SocketStreamWriter) =
            gateway_stream.split();

        // Wrap the writer in an Option so we can drop it when EOF is reached.
        let mut gateway_stream_tx: Option<SocketStreamWriter> = Some(gateway_stream_tx);

        let result: Result<(), ::anyhow::Error> = loop {
            tokio::select! {
                // Handle input from gateway.
                result = gateway_stream_rx.read(&mut gateway_buffer) => {
                    match result {
                        Ok(n) => {
                            if n == 0 {
                                break Ok(())
                            } else {
                                stdout.write_all(&gateway_buffer[..n]).await?;
                                stdout.flush().await?;
                            }
                        },
                        Err(error) => match error.kind() {
                            ErrorKind::UnexpectedEof | ErrorKind::ConnectionReset => {
                                warn!("gateway closed with {}: treating as normal close.", error.kind());
                                break Ok(());
                            },
                            _ => {
                                error!("failed to read from gateway: {}", error);
                                break Err(anyhow::anyhow!(error));
                            },
                        },
                    }
                },
                // Handle input from stdin thread.
                Some(data) = stdin_rx.recv() => {
                    if let Some(ref mut writer) = gateway_stream_tx {
                        if let Err(error) = writer.write_all(&data).await {
                            error!("failed to write to gateway: {}", error);
                            break Err(anyhow::anyhow!(error));
                        }
                    }
                },
                // Handle EOF from stdin.
                Some(()) = eof_rx.recv() => {
                    let mut eof_error: Option<::anyhow::Error> = None;
                    while let Ok(data) = stdin_rx.try_recv() {
                        if let Some(ref mut writer) = gateway_stream_tx {
                            if let Err(error) = writer.write_all(&data).await {
                                error!("failed to write remaining data to gateway: {}", error);
                                eof_error = Some(anyhow::anyhow!(error));
                                break;
                            }
                        }
                    }
                    if let Some(error) = eof_error {
                        break Err(error);
                    }
                    // Drop the gateway writer to signal EOF to the guest program.
                    gateway_stream_tx = None;
                },
                _ = signals.recv() => {
                    info!("received exit signal, stopping...");
                    break Ok(());
                }
            }
        };

        // Terminate the sandbox and retrieve the exit code.
        let exit_code: i32 = sandbox_cache.lock().await.kill(uservm_id).await?;

        // Send SIGUSR1 signal to stdin thread to interrupt the blocking read operation.
        // SAFETY: The thread ID is valid and was obtained from the stdin thread itself.
        let kill_result: i32 = unsafe { libc::pthread_kill(stdin_thread_id, SIGUSR1) };
        if kill_result != 0 {
            error!("failed to send signal to stdin thread: error code {kill_result}");
        }

        result.map(|()| exit_code)
    }

    ///
    /// # Description
    ///
    /// Thread function for reading from stdin in a blocking manner.
    ///
    /// # Parameters
    ///
    /// - `stdin_tx`: Channel sender to forward stdin data to the async task.
    /// - `thread_id_tx`: Channel sender to send the thread ID back to the main task.
    /// - `eof_tx`: Channel sender to notify when EOF is reached on stdin.
    ///
    fn stdin_thread(
        stdin_tx: UnboundedSender<Vec<u8>>,
        thread_id_tx: UnboundedSender<u64>,
        eof_tx: UnboundedSender<()>,
    ) {
        install_signal_handler();

        // Send thread ID back to the main task.
        // SAFETY: Calling pthread_self is safe as it only reads the thread ID.
        let thread_id: u64 = unsafe { libc::pthread_self() };
        if thread_id_tx.send(thread_id).is_err() {
            error!("failed to send thread ID: channel closed.");
            return;
        }

        let mut stdin: ::std::io::Stdin = ::std::io::stdin();
        let mut buffer: [u8; IO_BUFFER_SIZE] = [0; IO_BUFFER_SIZE];

        loop {
            match stdin.read(&mut buffer) {
                Ok(n) => {
                    if n == 0 {
                        let _ = eof_tx.send(());
                        break;
                    }
                    if stdin_tx.send(buffer[..n].to_vec()).is_err() {
                        break;
                    }
                },
                Err(error) => {
                    if error.kind() == ::std::io::ErrorKind::Interrupted {
                        info!("stdin thread interrupted by signal, exiting.");
                        break;
                    }
                    error!("failed to read from stdin: {error}");
                    break;
                },
            }
        }
    }

    ///
    /// # Description
    ///
    /// Retrieves the current user name from the operating system.
    ///
    /// # Returns
    ///
    /// Returns the current user name on success, or an error if the user name cannot be retrieved.
    ///
    fn get_current_user_name() -> Result<String> {
        let username: String = ::std::env::var("USER")
            .or_else(|_| ::std::env::var("USERNAME"))
            .map_err(|error| ::anyhow::anyhow!("failed to get current user name: {}", error))?;
        Ok(username)
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// No-op signal handler for SIGUSR1 used to interrupt blocking I/O operations in the stdin thread.
///
extern "C" fn stdin_thread_signal_handler(_: i32) {}

///
/// # Description
///
/// Installs signal handler for SIGUSR1 in the stdin thread.
///
// SAFETY:
// Pre-conditions:
// - The signal handler (`stdin_thread_signal_handler`) is a no-op and only sets EINTR on blocking
//   syscalls.
// - SIGUSR1 is not used for any other purpose in this process while this handler is installed.
// - The handler does not perform any non-signal-safe operations (it is an empty function).
// - The signal mask is empty, so no other signals are blocked during handler execution.
// - No SA_RESTART flag is set, so syscalls will return EINTR as intended.
// Post-conditions:
// - After installation, SIGUSR1 will interrupt blocking syscalls in the thread, causing them to
//   return EINTR.
// - Only this thread installs this handler for SIGUSR1; no other code should modify the handler
//   for SIGUSR1 while this is in effect.
// Invariants:
// - The handler remains a no-op and signal-safe.
// - The signal mask and flags remain as specified.
///
fn install_signal_handler() {
    // SAFETY: We install a signal handler that is a no-op so this is safe.
    let ret: c_int = unsafe {
        let sig_action: sigaction = sigaction {
            sa_sigaction: stdin_thread_signal_handler as *const () as usize,
            sa_mask: {
                let mut set: libc::sigset_t = mem::zeroed();
                sigemptyset(&mut set);
                set
            },
            sa_flags: 0,
            sa_restorer: None,
        };

        sigaction(INTERRUPT_SIGNAL, &sig_action, ptr::null_mut())
    };

    if ret != 0 {
        let errno: libc::c_int = unsafe { *libc::__errno_location() };
        error!("error installing signal handler (errno={errno:?})");
    }
}
