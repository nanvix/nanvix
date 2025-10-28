// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Terminal interface module for interactive mode.
//!
//! This module provides functionality to run programs in interactive mode, allowing users
//! to directly interact with guest binaries through a terminal interface. It handles
//! terminal raw mode, I/O streaming, and VM lifecycle management.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::nanvix_sandbox_cache::{
    SandboxCache,
    SandboxCacheConfig,
};
use ::std::{
    io::Read,
    sync::Arc,
};
use ::syscomm::{
    SocketStream,
    SocketStreamReader,
    SocketStreamWriter,
    SocketType,
    UnboundSocket,
    WriteAll,
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
    pub async fn run(
        &mut self,
        guest_binary_path: String,
        guest_binary_args: String,
    ) -> Result<()> {
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
                &guest_binary_path,
                &app_name,
                if guest_binary_args.is_empty() {
                    None
                } else {
                    Some(guest_binary_args)
                },
            )
            .await?;

        let gateway_stream: SocketStream = UnboundSocket::new(gateway_socket_type)
            .connect(&gateway_sockaddr)
            .await?;

        // Create channel for stdin data.
        let (stdin_tx, mut stdin_rx): (UnboundedSender<Vec<u8>>, UnboundedReceiver<Vec<u8>>) =
            mpsc::unbounded_channel();

        // Spawn a dedicated thread for blocking stdin reads. We use a separate thread because
        // tokio's async stdin handling is not suitable for standard blocking stdin reads.
        // Furthermore, we don't join this thread because it should run for the entire duration of
        // the terminal session.
        let _stdin_handle: ::std::thread::JoinHandle<()> = ::std::thread::spawn(move || {
            Self::stdin_thread(stdin_tx);
        });

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
    ///
    fn stdin_thread(stdin_tx: UnboundedSender<Vec<u8>>) {
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
                    error!("failed to read from stdin: {}", error);
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
