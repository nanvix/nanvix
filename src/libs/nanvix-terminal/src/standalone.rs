// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Standalone deployment mode implementation for the terminal.
//!
//! In standalone mode, the terminal directly drives a User VM instance via `StandaloneVmHandle`,
//! bypassing the sandbox cache, gateway sockets, and control-plane infrastructure. Guest I/O is
//! bridged through IKC channels: host stdin is forwarded to the guest's stdin, and guest stdout
//! is forwarded to the host's stdout.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    error,
    info,
    warn,
};
use ::std::io::Read;
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

/// Size of I/O buffers for terminal communication.
/// Set to 1 byte for character-by-character I/O to ensure responsive terminal interaction.
const IO_BUFFER_SIZE: usize = 1;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Configuration for the terminal in standalone mode.
///
/// Holds the minimal set of paths required to launch a User VM directly.
///
#[derive(Clone)]
pub struct TerminalConfig {
    /// Path to the guest kernel binary.
    kernel_binary_path: String,
    /// Optional path to a RAM filesystem image exposed to the guest.
    ramfs_filename: Option<String>,
    /// Optional file path for capturing guest stderr output.
    console_file: Option<String>,
    /// Optional GDB server port for debugging the guest.
    #[cfg(feature = "gdb")]
    gdb_port: Option<u16>,
}

impl TerminalConfig {
    ///
    /// # Description
    ///
    /// Creates a new terminal configuration.
    ///
    /// # Parameters
    ///
    /// - `kernel_binary_path`: Path to the guest kernel binary.
    /// - `ramfs_filename`: Optional path to a RAM filesystem image.
    /// - `console_file`: Optional file path for guest stderr capture.
    /// - `gdb_port`: Optional GDB server port.
    ///
    pub fn new(
        kernel_binary_path: String,
        ramfs_filename: Option<String>,
        console_file: Option<String>,
        #[cfg(feature = "gdb")] gdb_port: Option<u16>,
    ) -> Self {
        Self {
            kernel_binary_path,
            ramfs_filename,
            console_file,
            #[cfg(feature = "gdb")]
            gdb_port,
        }
    }
}

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
    config: TerminalConfig,
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
    pub fn new(config: TerminalConfig) -> Self {
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

        let initrd_args: Option<String> = if guest_binary_args.is_empty() {
            None
        } else {
            Some(guest_binary_args.to_string())
        };

        let (handle, io): (StandaloneVmHandle, StandaloneVmIo) = StandaloneVmHandle::spawn(
            self.config.kernel_binary_path.clone(),
            Some(guest_binary_path.to_string()),
            initrd_args,
            self.config.ramfs_filename.clone(),
            self.config.console_file.clone(),
            None,
            #[cfg(feature = "gdb")]
            self.config.gdb_port,
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
    /// Spawns a blocking thread for stdin reads and multiplexes guest output → host stdout and
    /// host stdin → guest input using `tokio::select!`.
    ///
    async fn bridge_io(io: StandaloneVmIo) -> Result<()> {
        let StandaloneVmIo {
            mut output_rx,
            input_tx,
        } = io;

        // Create channel for stdin data.
        let (stdin_tx, mut stdin_rx): (
            mpsc::UnboundedSender<Vec<u8>>,
            mpsc::UnboundedReceiver<Vec<u8>>,
        ) = mpsc::unbounded_channel();

        // Create channel for EOF notification.
        let (eof_tx, mut eof_rx): (mpsc::UnboundedSender<()>, mpsc::UnboundedReceiver<()>) =
            mpsc::unbounded_channel();

        // Spawn a dedicated thread for blocking stdin reads.
        let _stdin_handle: ::std::thread::JoinHandle<()> = ::std::thread::spawn(move || {
            Self::stdin_thread(stdin_tx, eof_tx);
        });

        let mut stdout: Stdout = io::stdout();

        loop {
            tokio::select! {
                // Handle output from guest VM.
                result = output_rx.recv() => {
                    match result {
                        Some(data) => {
                            stdout.write_all(&data).await?;
                            stdout.flush().await?;
                        },
                        None => {
                            // VM output channel closed — VM has exited.
                            break;
                        },
                    }
                },
                // Handle input from host stdin.
                Some(data) = stdin_rx.recv() => {
                    if input_tx.send(data).await.is_err() {
                        // Guest input channel closed — VM has exited.
                        break;
                    }
                },
                // Handle EOF from stdin.
                Some(()) = eof_rx.recv() => {
                    // Flush any remaining buffered stdin data.
                    while let Ok(data) = stdin_rx.try_recv() {
                        if input_tx.send(data).await.is_err() {
                            break;
                        }
                    }
                    // Drop input_tx to signal EOF to the guest.
                    drop(input_tx);
                    // Continue reading guest output until the VM exits.
                    while let Some(data) = output_rx.recv().await {
                        stdout.write_all(&data).await?;
                        stdout.flush().await?;
                    }
                    break;
                },
            }
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Thread function for reading from stdin in a blocking manner.
    ///
    /// # Parameters
    ///
    /// - `stdin_tx`: Channel sender to forward stdin data to the async task.
    /// - `eof_tx`: Channel sender to notify when EOF is reached on stdin.
    ///
    fn stdin_thread(stdin_tx: mpsc::UnboundedSender<Vec<u8>>, eof_tx: mpsc::UnboundedSender<()>) {
        let mut stdin: ::std::io::Stdin = ::std::io::stdin();
        let mut buffer: [u8; IO_BUFFER_SIZE] = [0; IO_BUFFER_SIZE];

        loop {
            match stdin.read(&mut buffer) {
                Ok(n) => {
                    if n == 0 {
                        // EOF reached, notify the main task.
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
}
