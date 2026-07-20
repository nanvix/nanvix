// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod http;
mod terminal;

//==================================================================================================
// Imports
//==================================================================================================

pub use self::{
    http::{
        NanvixdHttp,
        NanvixdHttpArgs,
    },
    terminal::{
        NanvixdTerminal,
        NanvixdTerminalArgs,
    },
};
use crate::{
    config::RunnerConfig,
    environment,
    warn_with_policy,
};
use ::anyhow::Result;
#[cfg(unix)]
use ::libc;
use ::log::{
    debug,
    error,
    trace,
};
use ::std::{
    path::{
        Path,
        PathBuf,
    },
    thread,
    time::Duration,
};
use ::tokio::{
    process::{
        Child,
        ChildStderr,
        ChildStdin,
        ChildStdout,
        Command,
    },
    runtime::{
        Builder,
        Handle,
        RuntimeFlavor,
    },
    task::block_in_place,
};

//==================================================================================================
// Nanvix Daemon Shared State
//==================================================================================================

///
/// Internal structure that encapsulates shared Nanvix Daemon lifecycle management.
///
struct Nanvixd {
    /// Child process handle for the Nanvix Daemon binary.
    cmd: Child,
    /// Maximum number of attempts to wait for a graceful shutdown.
    shutdown_attempts_max: usize,
    /// Delay (in milliseconds) between shutdown polling attempts.
    shutdown_retry_interval_ms: u64,
    /// Label describing the runtime context, used in log messages.
    context_label: String,
    /// Guard that sanitizes Nanvix artifacts when the daemon drops.
    _cleanup_guard: EnvironmentCleanupGuard,
}

impl Nanvixd {
    ///
    /// # Description
    ///
    /// Creates a new shared daemon handle from the spawned child process.
    ///
    /// # Parameters
    ///
    /// - `cmd`: Spawned Nanvix Daemon child process.
    /// - `shutdown_attempts_max`: Maximum number of graceful-shutdown polls.
    /// - `shutdown_retry_interval_ms`: Delay, in milliseconds, between polling attempts.
    /// - `context_label`: Label used when emitting log messages.
    /// - `cleanup_guard`: Guard that sanitizes Nanvix artifacts on drop.
    ///
    /// # Return Value
    ///
    /// Returns a `Nanvixd` handle initialized with the provided process metadata.
    ///
    fn new(
        cmd: Child,
        shutdown_attempts_max: usize,
        shutdown_retry_interval_ms: u64,
        context_label: &str,
        cleanup_guard: EnvironmentCleanupGuard,
    ) -> Self {
        Self {
            cmd,
            shutdown_attempts_max,
            shutdown_retry_interval_ms,
            context_label: context_label.to_string(),
            _cleanup_guard: cleanup_guard,
        }
    }

    ///
    /// # Description
    ///
    /// Builds the base command used to spawn the Nanvix Daemon, populating shared arguments.
    ///
    /// # Parameters
    ///
    /// - `config`: Runner configuration that provides binary paths.
    /// - `log_directory`: Directory where the Nanvix Daemon should persist component logs.
    ///
    /// # Return Value
    ///
    /// Returns a configured command ready for Nanvix Daemon-specific arguments.
    ///
    fn build_base_command(config: &RunnerConfig, log_directory: &Path) -> Command {
        let mut command: Command = Command::new(config.nanvixd_binary_path.as_str());
        command.current_dir(&config.working_directory);
        // Ensure the child process is killed if the Child handle is dropped without explicit
        // cleanup.  This acts as a best-effort safety net during normal unwinding and shutdown
        // paths where drop handlers run, helping to prevent orphaned processes.
        command.kill_on_drop(true);

        command.arg(::nanvixd::args::Args::OPT_LOG_DIRECTORY);
        command.arg(log_directory);

        command
    }

    ///
    /// # Description
    ///
    /// Returns a human-readable label describing the runtime context used in log messages.
    ///
    /// # Return Value
    ///
    /// Returns the context label string.
    ///
    fn context_label(&self) -> &str {
        self.context_label.as_str()
    }

    ///
    /// # Description
    ///
    /// Sends a Unix signal to the target Nanvix Daemon process.
    ///
    /// # Parameters
    ///
    /// - `signal`: Signal number to deliver.
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` if the signal is delivered; returns an error when delivery fails.
    ///
    #[cfg(unix)]
    fn signal(&self, signal: libc::c_int) -> Result<()> {
        let context: String = self.context_label().to_string();
        trace!("signal(): signal={signal}, context={context}");

        // Get Nanvix Daemon PID.
        let Some(pid) = self.cmd.id() else {
            let reason: String = "nanvixd pid is unavailable".to_string();
            error!("signal(): {reason} (signal={signal}, context={context})");
            return Err(::anyhow::anyhow!(reason));
        };

        // Try to convert PID.
        let pid: libc::pid_t = match pid.try_into() {
            Err(error) => {
                let reason: String = format!(
                    "failed to convert nanvixd pid (error={error}, signal={signal}, pid={pid}, \
                     context={context})"
                );
                error!("signal(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
            Ok(pid) => pid,
        };

        // Send signal to the Nanvix Daemon process and check for errors.
        debug!("signal(): sending {signal} to nanvixd (pid={pid}, context={context})");
        let ret: libc::c_int = unsafe { libc::kill(pid, signal) };
        if ret != 0 {
            let os_error: ::std::io::Error = ::std::io::Error::last_os_error();
            let reason: String = format!(
                "failed to send {signal} to nanvixd (pid={pid}, errno={os_error}, \
                 context={context})"
            );
            error!("signal(): {reason}");
            return Err(::anyhow::anyhow!(reason));
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Waits for the Nanvix Daemon process to exit.
    ///
    /// This function retries at fixed intervals until either the process terminates or the
    /// configured number of attempts is exhausted.
    ///
    /// # Parameters
    ///
    /// - `sleep_duration`: Duration to sleep between successive `try_wait_exit()` calls.
    /// - `max_attempts`: Maximum number of polling attempts before giving up.
    ///
    /// # Return Value
    ///
    /// Returns `true` if the process exits before the attempt limit, `false` if it is still
    /// running after all attempts, and an error if the wait operation itself fails.
    ///
    fn try_wait_exit(&mut self, sleep_duration: Duration, max_attempts: usize) -> Result<bool> {
        let mut attempts: usize = 0;
        let context: String = self.context_label().to_string();
        loop {
            match self.cmd.try_wait() {
                Err(error) => {
                    let reason: String =
                        format!("failed to wait for nanvixd (error={error}, context={context})");
                    error!("try_wait_exit(): {reason}");
                    return Err(::anyhow::anyhow!(reason));
                },
                Ok(Some(_status)) => {
                    debug!(
                        "try_wait_exit(): nanvixd process exited after {attempts} attempts \
                         (context={context})"
                    );
                    return Ok(true);
                },
                Ok(None) => {
                    if attempts >= max_attempts {
                        let reason: String = format!(
                            "nanvixd process failed to exit after {attempts} attempts \
                             (context={context})"
                        );
                        debug!("try_wait_exit(): {reason}");
                        return Ok(false);
                    }

                    attempts += 1;
                    thread::sleep(sleep_duration);
                },
            }
        }
    }

    ///
    ///
    /// # Description
    ///
    /// Takes ownership of the stdin handle when the daemon is running in interactive mode.
    ///
    /// # Return Value
    ///
    /// Returns the stdin handle if available; otherwise returns `None` when stdin was already
    /// taken.
    ///
    fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.cmd.stdin.take()
    }

    ///
    /// # Description
    ///
    /// Takes ownership of the stdout handle for interactive mode consumers.
    ///
    /// # Return Value
    ///
    /// Returns the stdout handle if it exists; otherwise returns `None` when previously taken.
    ///
    fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.cmd.stdout.take()
    }

    ///
    /// # Description
    ///
    /// Takes ownership of the stderr handle for interactive mode consumers.
    ///
    /// # Return Value
    ///
    /// Returns the stderr handle if it exists; otherwise returns `None` when previously taken.
    ///
    fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.cmd.stderr.take()
    }

    ///
    /// # Description
    ///
    /// Waits for the process to exit and returns its exit code.
    ///
    /// # Return Value
    ///
    /// Returns the exit code of the process on success; returns an error if waiting fails or
    /// the process was terminated by a signal.
    ///
    async fn wait_exit_code(&mut self) -> Result<i32> {
        let context: String = self.context_label().to_string();
        trace!("wait_exit_code(): context={context}");

        match self.cmd.wait().await {
            Ok(status) => {
                if let Some(code) = status.code() {
                    debug!("wait_exit_code(): process exited with code {code} (context={context})");
                    Ok(code)
                } else {
                    let reason: String = format!(
                        "process terminated by signal without exit code (context={context})"
                    );
                    error!("wait_exit_code(): {reason}");
                    Err(::anyhow::anyhow!(reason))
                }
            },
            Err(error) => {
                let reason: String =
                    format!("failed to wait for process exit (context={context}, error={error})");
                error!("wait_exit_code(): {reason}");
                Err(::anyhow::anyhow!(reason))
            },
        }
    }
}

impl Drop for Nanvixd {
    ///
    /// # Description
    ///
    /// Cleans up the Nanvix Daemon process by attempting a graceful shutdown via SIGINT,
    /// followed by a forced shutdown via SIGKILL if the former fails.
    ///
    /// On Windows, the process is terminated directly via `start_kill()` (TerminateProcess)
    /// because Unix signals are not available.
    ///
    /// # Return Value
    ///
    /// Returns `()`; logs errors when cleanup attempts fail.
    ///
    fn drop(&mut self) {
        let context: String = self.context_label().to_string();
        match self.cmd.try_wait() {
            Ok(Some(_status)) => {
                trace!(
                    "drop(): skipping cleanup because process already exited (context={context})"
                );
                return;
            },
            Ok(None) => {
                trace!("drop(): context={context}");
            },
            Err(error) => {
                warn_with_policy!(
                    "drop(): failed to probe nanvixd status before cleanup (context={context}, \
                     error={error})"
                );
                trace!("drop(): context={context}");
            },
        }

        let wait_duration: Duration = Duration::from_millis(self.shutdown_retry_interval_ms);
        let max_attempts: usize = self.shutdown_attempts_max;

        match self.try_wait_exit(wait_duration, max_attempts) {
            Err(error) => {
                warn_with_policy!(
                    "drop(): failed to wait for nanvixd exit before sending signals \
                     (context={context}, error={error})"
                );
            },
            Ok(true) => {
                debug!(
                    "drop(): nanvixd exited while waiting for natural shutdown (context={context})"
                );
                return;
            },
            Ok(false) => {
                trace!(
                    "drop(): nanvixd still running after natural shutdown wait (context={context})"
                );
            },
        }

        #[cfg(unix)]
        {
            // Send SIGINT for graceful shutdown and check for errors.
            let sigint_sent: bool = match self.signal(libc::SIGINT) {
                Err(error) => {
                    error!(
                        "drop(): failed to send SIGINT to nanvixd (context={context}, \
                         error={error})"
                    );
                    false
                },
                Ok(()) => true,
            };

            // Try to wait for graceful shutdown.
            match self.try_wait_exit(wait_duration, max_attempts) {
                Err(error) => {
                    error!("drop(): SIGINT wait failed (error={error})");
                },
                Ok(exited) => {
                    if exited {
                        debug!("drop(): nanvixd exited gracefully after SIGINT");
                        return;
                    }
                    if sigint_sent {
                        warn_with_policy!(
                            "drop(): nanvixd did not exit after SIGINT, sending SIGKILL \
                             (context={context})"
                        );
                    } else {
                        warn_with_policy!(
                            "drop(): nanvixd still running and SIGINT could not be delivered, \
                             sending SIGKILL (context={context})"
                        );
                    }
                },
            }

            // Send SIGKILL for forced shutdown and check for errors.
            if let Err(error) = self.signal(libc::SIGKILL) {
                error!(
                    "drop(): failed to send SIGKILL to nanvixd (context={context}, error={error})"
                );
                return;
            }

            // Try to wait for forced shutdown.
            match self.try_wait_exit(wait_duration, max_attempts) {
                Err(error) => {
                    error!("drop(): SIGKILL wait failed (error={error})");
                },
                Ok(exited) => {
                    if exited {
                        debug!("drop(): nanvixd exited after SIGKILL (context={context})");
                    } else {
                        error!("drop(): nanvixd failed to exit after SIGKILL (context={context})");
                    }
                },
            }
        }

        #[cfg(not(unix))]
        {
            // On non-Unix platforms, terminate the process forcefully. This calls
            // TerminateProcess on Windows, which is the closest equivalent to SIGKILL.
            debug!("drop(): terminating nanvixd process (context={context})");
            if let Err(error) = self.cmd.start_kill() {
                error!("drop(): failed to terminate nanvixd (context={context}, error={error})");
                return;
            }

            match self.try_wait_exit(wait_duration, max_attempts) {
                Err(error) => {
                    error!(
                        "drop(): wait after terminate failed (context={context}, error={error})"
                    );
                },
                Ok(exited) => {
                    if exited {
                        debug!("drop(): nanvixd terminated successfully (context={context})");
                    } else {
                        error!(
                            "drop(): nanvixd failed to exit after terminate (context={context})"
                        );
                    }
                },
            }
        }
    }
}

//==================================================================================================
// Environment Cleanup Guard
//==================================================================================================

///
/// # Description
///
/// Helper that guarantees host cleanup runs even when the Nanvixd drop handler exits early.
struct EnvironmentCleanupGuard {
    /// Directory sanitized when removing Nanvix artifacts.
    tmp_directory: PathBuf,
}

impl EnvironmentCleanupGuard {
    ///
    /// # Description
    ///
    /// Creates a new guard that sanitizes the host environment when dropped.
    ///
    /// # Parameters
    ///
    /// - `tmp_directory`: Directory sanitized when removing Nanvix artifacts.
    fn new(tmp_directory: PathBuf) -> Self {
        Self { tmp_directory }
    }
}

impl Drop for EnvironmentCleanupGuard {
    fn drop(&mut self) {
        let tmp_directory: PathBuf = self.tmp_directory.clone();

        if let Ok(handle) = Handle::try_current() {
            match handle.runtime_flavor() {
                RuntimeFlavor::MultiThread => {
                    let tmp_clone: PathBuf = tmp_directory.clone();
                    block_in_place(|| {
                        handle.block_on(async {
                            environment::cleanup_after_run(tmp_clone.as_path()).await;
                        });
                    });
                },
                RuntimeFlavor::CurrentThread => {
                    let tmp_clone: PathBuf = tmp_directory.clone();
                    handle.spawn(async move {
                        environment::cleanup_after_run(tmp_clone.as_path()).await;
                    });
                },
                _ => {
                    let tmp_clone: PathBuf = tmp_directory.clone();
                    warn_with_policy!(
                        "EnvironmentCleanupGuard::drop(): unknown runtime flavor, running cleanup \
                         synchronously"
                    );
                    block_in_place(|| {
                        handle.block_on(async {
                            environment::cleanup_after_run(tmp_clone.as_path()).await;
                        });
                    });
                },
            }
            return;
        }

        match Builder::new_current_thread().enable_all().build() {
            Ok(runtime) => {
                runtime.block_on(async {
                    environment::cleanup_after_run(tmp_directory.as_path()).await;
                });
            },
            Err(error) => warn_with_policy!(
                "EnvironmentCleanupGuard::drop(): failed to build cleanup runtime (error={})",
                error
            ),
        }
    }
}
