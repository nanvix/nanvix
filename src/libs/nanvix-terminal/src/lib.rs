// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Terminal interface module for interactive mode.
//!
//! This module provides functionality to run programs in interactive mode, allowing users
//! to directly interact with guest binaries through a terminal interface. It handles
//! terminal raw mode, I/O streaming, and VM lifecycle management.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

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
use ::nanvix_sandbox_cache::{
    syscomm::{
        SocketStream,
        SocketStreamReader,
        SocketStreamWriter,
        SocketType,
        UnboundSocket,
        WriteAll,
    },
    SandboxCache,
    SandboxCacheConfig,
};
use ::std::{
    io::Read,
    mem,
    ptr,
    sync::Arc,
};
use ::syslog::error;
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
use ::user_vm_api::UserVmIdentifier;

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
/// Terminal interface for interacting with user VMs.
///
pub struct Terminal {
    /// Configuration for sandbox cache management.
    config: SandboxCacheConfig,
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
    /// - `config`: Configuration for sandbox cache management.
    ///
    pub fn new(config: SandboxCacheConfig) -> Self {
        Self { config }
    }

    ///
    /// # Description
    ///
    /// Runs the terminal interface.
    ///
    /// # Parameters
    ///
    /// - `guest_binary_path`: Path to the guest binary to execute.
    /// - `guest_binary_args`: Arguments to pass to the guest binary.
    ///
    /// # Returns
    ///
    /// On success, this function returns an empty tuple after the terminal session ends. On
    /// failure, it returns an object that describes the error that occurred.
    ///
    pub async fn run(&mut self, guest_binary_path: &str, guest_binary_args: &str) -> Result<()> {
        let sandbox_cache: Arc<Mutex<SandboxCache>> = SandboxCache::new(self.config.clone());
        let mut signals: Signal = signal(SignalKind::interrupt())?;

        let tenant_id: String = Self::get_current_user_name()?;
        let app_name: String = DEFAULT_APP_NAME.to_string();
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

        // Create channel for thread ID communication.
        let (thread_id_tx, mut thread_id_rx): (UnboundedSender<u64>, UnboundedReceiver<u64>) =
            mpsc::unbounded_channel();

        // Spawn a dedicated thread for blocking stdin reads. We use a separate thread because
        // tokio's async stdin handling is not suitable for standard blocking stdin reads.
        // Furthermore, we don't join this thread because it should run for the entire duration of
        // the terminal session.
        let _stdin_handle: ::std::thread::JoinHandle<()> = ::std::thread::spawn(move || {
            Self::stdin_thread(stdin_tx, thread_id_tx);
        });

        // Wait for the thread ID to be sent.
        let stdin_thread_id: u64 = thread_id_rx.recv().await.ok_or_else(|| {
            let reason: &str = "failed to receive id of stdin thread";
            error!("{reason}");
            anyhow::anyhow!(reason)
        })?;

        let mut stdout: Stdout = io::stdout();
        let mut gateway_buffer: [u8; IO_BUFFER_SIZE] = [0; IO_BUFFER_SIZE];

        let (mut gateway_stream_rx, mut gateway_stream_tx): (
            SocketStreamReader,
            SocketStreamWriter,
        ) = gateway_stream.split();

        let result: Result<(), ::anyhow::Error> = loop {
            tokio::select! {
                // Handle input from gateway.
                result = gateway_stream_rx.read(&mut gateway_buffer) => {
                    match result {
                        Ok(n) => {
                            if n == 0 {
                                // Connection closed.
                                break Ok(())
                            } else {
                                // Echo character to terminal.
                                stdout.write_all(&gateway_buffer[..n]).await?;
                                stdout.flush().await?;
                            }
                        },
                        Err(error) => {
                            error!("failed to read from gateway: {}", error);
                            break Err(anyhow::anyhow!(error));
                        },
                    }
                },
                // Handle input from stdin thread.
                Some(data) = stdin_rx.recv() => {
                    // Send data to gateway.
                    if let Err(error) = gateway_stream_tx.write_all(&data).await {
                        error!("failed to write to gateway: {}", error);
                        break Err(anyhow::anyhow!(error));
                    }
                },
                _ = signals.recv() => {
                    break Ok(());
                }

            }
        };

        // Shutdown VM.
        if let Err(error) = sandbox_cache.lock().await.kill(uservm_id).await {
            error!("failed to shutdown VM: {error}");
        }

        // Send SIGUSR1 signal to stdin thread to interrupt the blocking read operation.
        // SAFETY: The thread ID is valid and was obtained from the stdin thread itself.
        let kill_result: i32 = unsafe { libc::pthread_kill(stdin_thread_id, SIGUSR1) };
        if kill_result != 0 {
            error!("failed to send signal to stdin thread: error code {kill_result}");
        }

        result
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
    ///
    fn stdin_thread(stdin_tx: UnboundedSender<Vec<u8>>, thread_id_tx: UnboundedSender<u64>) {
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
                        // EOF reached.
                        break;
                    }
                    // Send data to async task.
                    if stdin_tx.send(buffer[..n].to_vec()).is_err() {
                        // Channel closed, exit thread.
                        break;
                    }
                },
                Err(error) => {
                    // Check if operation was interrupted by a signal.
                    if error.kind() == ::std::io::ErrorKind::Interrupted {
                        // Signal received, exit gracefully.
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
/// When SIGUSR1 is delivered, this handler causes any blocking system calls (such as read)
/// to be interrupted and return EINTR, allowing the thread to exit gracefully or handle the
/// interruption as needed. The handler itself performs no action.
///
extern "C" fn stdin_thread_signal_handler(_: i32) {}

///
/// # Description
///
/// Installs signal handler for SIGUSR1 in the stdin thread.
///
// SAFETY:
// Pre-conditions:
// - The signal handler (`stdin_thread_signal_handler`) is a no-op and only sets EINTR on blocking syscalls.
// - SIGUSR1 is not used for any other purpose in this process while this handler is installed.
// - The handler does not perform any non-signal-safe operations (it is an empty function).
// - The signal mask is empty, so no other signals are blocked during handler execution.
// - No SA_RESTART flag is set, so syscalls will return EINTR as intended.
// Post-conditions:
// - After installation, SIGUSR1 will interrupt blocking syscalls in the thread, causing them to return EINTR.
// - Only this thread installs this handler for SIGUSR1; no other code should modify the handler for SIGUSR1 while this is in effect.
// Invariants:
// - The handler remains a no-op and signal-safe.
// - The signal mask and flags remain as specified.
/// EINTR. This allows graceful shutdown of the stdin thread.
///
///
fn install_signal_handler() {
    // SAFETY: We install a signal handler that is a no-op so this is safe.
    let ret: c_int = unsafe {
        let sig_action: sigaction = sigaction {
            sa_sigaction: stdin_thread_signal_handler as usize,
            // Empty set to not block any other signals that may happen during signal handling.
            sa_mask: {
                let mut set: libc::sigset_t = mem::zeroed();
                sigemptyset(&mut set);
                set
            },
            // No SA_RESTART so that syscall will return EINTR.
            sa_flags: 0,
            sa_restorer: None,
        };

        sigaction(INTERRUPT_SIGNAL, &sig_action, ptr::null_mut())
    };

    if ret != 0 {
        // Notify the error, but don't fail.
        let errno: libc::c_int = unsafe { *libc::__errno_location() };
        error!("error installing signal handler (errno={errno:?})");
    }
}
