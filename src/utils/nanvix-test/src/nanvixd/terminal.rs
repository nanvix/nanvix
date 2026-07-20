// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    EnvironmentCleanupGuard,
    Nanvixd,
};
use crate::config::RunnerConfig;
use ::anyhow::Result;
use ::log::{
    debug,
    error,
    trace,
};
use ::no_fail::no_fail;
use ::std::{
    path::{
        Path,
        PathBuf,
    },
    process::Stdio,
};
use ::tokio::process::{
    ChildStderr,
    ChildStdin,
    ChildStdout,
    Command,
};

//==================================================================================================
// Nanvix Daemon Terminal Handle
//==================================================================================================

///
/// Handle to a Nanvix Daemon instance running in interactive terminal mode.
///
pub struct NanvixdTerminal {
    /// Shared process management state.
    inner: Nanvixd,
}

impl NanvixdTerminal {
    ///
    /// # Description
    ///
    /// Spawns a Nanvix Daemon instance configured for interactive coordination over stdin/stdout.
    ///
    /// # Parameters
    ///
    /// - `config`: Configuration object describing how the Nanvix Daemon should be spawned.
    /// - `args`: Runtime arguments used when spawning the Nanvix Daemon.
    ///
    /// # Return Value
    ///
    /// Returns a handle to the interactive Nanvix Daemon on success; returns an error if the
    /// process cannot be spawned.
    ///
    pub async fn spawn(config: &RunnerConfig, args: &NanvixdTerminalArgs) -> Result<Self> {
        let log_directory: &Path = args.log_directory();
        trace!(
            "spawn(): nanvixd_binary_path={}, working_directory={}, toolchain_path={}, mode={}",
            config.nanvixd_binary_path,
            config.working_directory,
            config.toolchain_path,
            args.mode_label(),
        );

        let program_path: &str = args.program_path();
        let program_args: &[String] = args.program_args();

        let mut command: Command = Nanvixd::build_base_command(config, log_directory);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // Append extra command-line arguments passed directly to nanvixd.
        for extra_nanvixd_arg in args.extra_nanvixd_args() {
            command.arg(extra_nanvixd_arg);
        }

        command.arg(::nanvixd::args::Args::OPT_SEPARATOR);
        command.arg(program_path);
        command.args(program_args);

        let mode_label: &str = "mode=interactive";

        // Spawn the Nanvix Daemon process.
        let nanvixd: Self = match command.spawn() {
            Err(error) => {
                let reason: String = error.to_string();
                error!("spawn(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
            Ok(child) => {
                // We cannot fail here; otherwise, `child` would linger.
                no_fail!(Self, {
                    debug!("spawn(): nanvixd spawned with pid {}", child.id().unwrap_or(0));
                    let cleanup_guard: EnvironmentCleanupGuard =
                        EnvironmentCleanupGuard::new(PathBuf::from(config.tmp_directory.as_str()));
                    Ok(Self {
                        inner: Nanvixd::new(
                            child,
                            config.nanvixd_shutdown_attempts_max,
                            config.nanvixd_shutdown_retry_interval_ms,
                            mode_label,
                            cleanup_guard,
                        ),
                    })
                })
            },
        };

        Ok(nanvixd)
    }

    ///
    ///
    /// # Description
    ///
    /// Takes ownership of stdin for piping data into the interactive daemon.
    ///
    /// # Return Value
    ///
    /// Returns the stdin handle when available; otherwise returns `None` if stdin has already
    /// been taken.
    ///
    pub fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.inner.take_stdin()
    }

    ///
    /// # Description
    ///
    /// Takes ownership of the stdout handle for interactive consumers.
    ///
    /// # Return Value
    ///
    /// Returns the stdout handle if it has not been taken yet; otherwise returns `None`.
    ///
    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.inner.take_stdout()
    }

    ///
    /// # Description
    ///
    /// Takes ownership of the stderr handle for interactive consumers.
    ///
    /// # Return Value
    ///
    /// Returns the stderr handle if it is still available; otherwise returns `None`.
    ///
    pub fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.inner.take_stderr()
    }

    ///
    /// # Description
    ///
    /// Waits for the Nanvix Daemon process to exit and returns its exit code.
    ///
    /// # Return Value
    ///
    /// Returns the exit code of the process on success; returns an error if waiting fails or
    /// the process was terminated by a signal.
    ///
    pub async fn wait_exit_code(&mut self) -> Result<i32> {
        self.inner.wait_exit_code().await
    }
}

//==================================================================================================
// Nanvix Daemon Terminal Arguments
//==================================================================================================

///
/// Bundles the runtime parameters required when spawning the Nanvix Daemon in interactive
/// terminal mode.
///
pub struct NanvixdTerminalArgs {
    /// Program executed by the daemon when running interactively.
    program_path: String,
    /// Command-line arguments forwarded to the interactive workload.
    program_args: Vec<String>,
    /// Directory where Nanvix Daemon components should emit logs.
    log_directory: PathBuf,
    /// Command-line arguments passed directly to nanvixd.
    extra_nanvixd_args: Vec<String>,
}

impl NanvixdTerminalArgs {
    ///
    /// # Description
    ///
    /// Builds the runtime bundle needed to launch the Nanvix Daemon in interactive mode.
    /// Log files are managed by the caller when capturing stdout/stderr streams.
    ///
    /// # Parameters
    ///
    /// - `program_path`: Path to the workload that should execute inside the sandbox.
    /// - `program_args`: Command-line arguments forwarded to the workload.
    /// - `log_directory`: Directory where the Nanvix Daemon should emit component logs.
    /// - `extra_nanvixd_args`: Command-line arguments passed directly to nanvixd.
    ///
    /// # Return Value
    ///
    /// Returns a ready-to-use argument bundle because interactive mode does not touch the
    /// filesystem during construction.
    ///
    pub fn new(
        program_path: &str,
        program_args: &[String],
        log_directory: &Path,
        extra_nanvixd_args: &[String],
    ) -> Result<Self> {
        Ok(Self {
            program_path: program_path.to_string(),
            program_args: program_args.to_vec(),
            log_directory: log_directory.to_path_buf(),
            extra_nanvixd_args: extra_nanvixd_args.to_vec(),
        })
    }

    ///
    /// # Description
    ///
    /// Returns the interactive program path forwarded to the Nanvix Daemon.
    ///
    /// # Return Value
    ///
    /// Returns a reference to the program path string.
    ///
    pub fn program_path(&self) -> &str {
        self.program_path.as_str()
    }

    ///
    /// # Description
    ///
    /// Returns the interactive program arguments forwarded to the Nanvix Daemon.
    ///
    /// # Return Value
    ///
    /// Returns a slice containing the program arguments.
    ///
    pub fn program_args(&self) -> &[String] {
        self.program_args.as_slice()
    }

    ///
    /// # Description
    ///
    /// Returns the directory where component logs should be persisted.
    ///
    /// # Return Value
    ///
    /// Returns the component log directory as a path reference.
    ///
    pub fn log_directory(&self) -> &Path {
        self.log_directory.as_path()
    }

    ///
    /// # Description
    ///
    /// Returns a textual label describing the launch mode (used for logging).
    ///
    /// # Return Value
    ///
    /// Returns the static label that describes interactive mode.
    ///
    pub fn mode_label(&self) -> &'static str {
        "mode=interactive"
    }

    ///
    /// # Description
    ///
    /// Returns the command-line arguments passed directly to nanvixd.
    ///
    /// # Return Value
    ///
    /// Returns a slice containing the nanvixd arguments.
    ///
    pub fn extra_nanvixd_args(&self) -> &[String] {
        self.extra_nanvixd_args.as_slice()
    }
}
