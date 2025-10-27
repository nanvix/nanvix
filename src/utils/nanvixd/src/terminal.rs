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
use ::std::sync::Arc;
use ::syscomm::{
    SocketStream,
    SocketStreamReader,
    SocketStreamWriter,
    SocketType,
    UnboundSocket,
    WriteAll,
};
use ::syslog::{
    error,
    info,
};
use ::tokio::{
    io::{
        self,
        AsyncReadExt,
        AsyncWriteExt,
        Stdin,
        Stdout,
    },
    sync::Mutex,
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

///
/// # Description
///
/// RAII guard for terminal raw mode. Restores terminal to original mode on drop.
///
struct RawModeGuard {
    /// Original terminal settings.
    original_termios: ::libc::termios,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RawModeGuard {
    ///
    /// # Description
    ///
    /// Enables raw mode for stdin.
    ///
    /// # Returns
    ///
    /// Returns a guard that will restore the original terminal mode on drop.
    ///
    fn new() -> Result<Self> {
        // SAFETY: We are accessing stdin's file descriptor (STDIN_FILENO) which is always
        // valid for the current process. The termios structure is properly zeroed before use,
        // and tcgetattr/tcsetattr are standard POSIX calls that safely manipulate terminal
        // attributes through the provided file descriptor.
        unsafe {
            let mut termios: ::libc::termios = ::std::mem::zeroed();
            if ::libc::tcgetattr(::libc::STDIN_FILENO, &mut termios) != 0 {
                let error: ::std::io::Error = ::std::io::Error::last_os_error();
                error!("failed to get terminal attributes: {}", error);
                return Err(anyhow::anyhow!(error));
            }

            let original_termios: ::libc::termios = termios;

            // Disable canonical mode and echo.
            termios.c_lflag &= !(::libc::ICANON | ::libc::ECHO);
            // Set minimum characters to read to 0 (non-blocking).
            termios.c_cc[::libc::VMIN] = 0;
            // Set timeout to 1 decisecond (100ms).
            termios.c_cc[::libc::VTIME] = 1;

            if ::libc::tcsetattr(::libc::STDIN_FILENO, ::libc::TCSANOW, &termios) != 0 {
                let error: ::std::io::Error = ::std::io::Error::last_os_error();
                error!("failed to set terminal attributes: {}", error);
                return Err(anyhow::anyhow!(error));
            }

            Ok(Self { original_termios })
        }
    }
}

impl Drop for RawModeGuard {
    ///
    /// # Description
    ///
    /// Restores the original terminal mode.
    ///
    fn drop(&mut self) {
        // SAFETY: We are restoring the terminal attributes to their original state.
        // The original_termios was obtained through a valid tcgetattr call in new().
        unsafe {
            ::libc::tcsetattr(::libc::STDIN_FILENO, ::libc::TCSANOW, &self.original_termios);
        }
    }
}

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

        let (mut gateway_stream_rx, mut gateway_stream_tx): (
            SocketStreamReader,
            SocketStreamWriter,
        ) = gateway_stream.split();

        // Enable raw mode for terminal.
        let _raw_mode_guard: RawModeGuard = RawModeGuard::new()?;

        let mut stdout: Stdout = io::stdout();
        let mut stdin: Stdin = io::stdin();
        let mut stdin_buffer: [u8; IO_BUFFER_SIZE] = [0; IO_BUFFER_SIZE];
        let mut gateway_buffer: [u8; IO_BUFFER_SIZE] = [0; IO_BUFFER_SIZE];

        let result: Result<()> = loop {
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
                    // Handle input from terminal.
                    result = stdin.read(&mut stdin_buffer) => {
                        match result {
                            Ok(n) => {
                                if n == 0 {
                                    // EOF reached.
                                    break Ok(());
                                } else {
                                    // Send character to gateway.
                                    if let Err(error) = gateway_stream_tx.write_all(&stdin_buffer[..n]).await {
                                        error!("failed to write to gateway: {}", error);
                                        break Err(anyhow::anyhow!(error));
                                    }
                                }
                            },
                            Err(error) => {
                                error!("failed to read from terminal: {}", error);
                                break Err(anyhow::anyhow!(error));
                            },
                        }
                    },
                    // Handle interrupt signal (Ctrl+C).
                    _ = tokio::signal::ctrl_c() => {
                        info!("received interrupt signal, exiting terminal");
                        break Ok(())
                    }
            }
        };

        // Shutdown VM.
        if let Err(e) = sandbox_cache.lock().await.kill(uservm_id).await {
            error!("failed to shutdown VM: {}", e);
        }

        result
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
    pub fn get_current_user_name() -> Result<String> {
        let username: String = ::std::env::var("USER")
            .or_else(|_| ::std::env::var("USERNAME"))
            .map_err(|error| ::anyhow::anyhow!("failed to get current user name: {}", error))?;
        Ok(username)
    }
}
