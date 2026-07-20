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
    fs::{
        File,
        OpenOptions,
    },
    path::{
        Path,
        PathBuf,
    },
    process::Stdio,
};
use ::tokio::{
    net::TcpStream,
    process::Command,
    time::sleep,
};

//==================================================================================================
// Nanvix Daemon HTTP Handle
//==================================================================================================

///
/// Handle to a Nanvix Daemon instance exposing the HTTP control interface.
///
pub struct NanvixdHttp {
    /// Shared process management state.
    inner: Nanvixd,
    /// IPv4 address exposed by the running daemon.
    ipv4_addr: String,
    /// TCP port exposed by the running daemon.
    port_num: u16,
    /// Maximum number of readiness checks before giving up on the HTTP endpoint.
    nanvixd_ready_attempts_max: usize,
    /// Interval (in milliseconds) between readiness probes.
    nanvixd_ready_retry_interval_ms: u64,
}

impl NanvixdHttp {
    ///
    /// # Description
    ///
    /// Spawns a new Nanvix Daemon instance configured for HTTP coordination.
    ///
    /// # Parameters
    ///
    /// - `config`: Configuration object describing how the Nanvix Daemon should be spawned.
    /// - `args`: File handles and runtime arguments used when spawning the Nanvix Daemon.
    ///
    /// # Return Value
    ///
    /// Returns a handle to the HTTP-enabled Nanvix Daemon on success; returns an error when the
    /// child process cannot be spawned or the readiness checks fail.
    ///
    pub async fn spawn(config: &RunnerConfig, args: &NanvixdHttpArgs) -> Result<Self> {
        let log_directory: &Path = args.log_directory();
        trace!(
            "spawn(): nanvixd_binary_path={}, working_directory={}, toolchain_path={}, mode=http",
            config.nanvixd_binary_path, config.working_directory, config.toolchain_path,
        );

        let port_num: u16 = args.port_num();
        let http_address: String = format!("{}:{}", args.ipv4_addr(), port_num);

        let mut command: Command = Nanvixd::build_base_command(config, log_directory);

        let stdout_file: File = args.stdout_file_handle().try_clone().map_err(|error| {
            let reason: String = format!("failed to clone nanvixd stdout log file (error={error})");
            error!("spawn(): {reason}");
            ::anyhow::anyhow!(reason)
        })?;

        let stderr_file: File = args.stderr_file_handle().try_clone().map_err(|error| {
            let reason: String = format!("failed to clone nanvixd stderr log file (error={error})");
            error!("spawn(): {reason}");
            ::anyhow::anyhow!(reason)
        })?;

        command.stdin(Stdio::null());
        command.stdout(Stdio::from(stdout_file));
        command.stderr(Stdio::from(stderr_file));
        command.arg(::nanvixd::args::Args::OPT_HTTP_SOCKADDR);
        command.arg(http_address.as_str());

        // Append command-line arguments passed directly to nanvixd.
        for extra_nanvixd_arg in args.extra_nanvixd_args() {
            command.arg(extra_nanvixd_arg);
        }

        let mode_label: String = format!("http_address={http_address}");

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
                    let ipv4_addr: String = args.ipv4_addr().to_string();
                    let cleanup_guard: EnvironmentCleanupGuard =
                        EnvironmentCleanupGuard::new(PathBuf::from(config.tmp_directory.as_str()));

                    Ok(Self {
                        inner: Nanvixd::new(
                            child,
                            config.nanvixd_shutdown_attempts_max,
                            config.nanvixd_shutdown_retry_interval_ms,
                            mode_label.as_str(),
                            cleanup_guard,
                        ),
                        ipv4_addr,
                        port_num,
                        nanvixd_ready_attempts_max: config.nanvixd_ready_attempts_max,
                        nanvixd_ready_retry_interval_ms: config.nanvixd_ready_retry_interval_ms,
                    })
                })
            },
        };

        nanvixd.try_wait_ready().await?;

        Ok(nanvixd)
    }

    ///
    /// # Description
    ///
    /// Waits until the Nanvix Daemon accepts TCP connections on the configured HTTP socket.
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` once the daemon becomes reachable; returns an error if the readiness
    /// attempts are exhausted without success.
    ///
    async fn try_wait_ready(&self) -> Result<()> {
        let http_endpoint: String = self.http_address();
        for attempt in 0..self.nanvixd_ready_attempts_max {
            match TcpStream::connect(http_endpoint.as_str()).await {
                Ok(stream) => {
                    drop(stream);
                    debug!(
                        "try_wait_ready(): nanvixd reachable (attempt={}, context={})",
                        attempt + 1,
                        self.inner.context_label()
                    );
                    return Ok(());
                },
                Err(error) => {
                    debug!(
                        "try_wait_ready(): attempt {} failed (context={}, error={error})",
                        attempt + 1,
                        self.inner.context_label()
                    );
                },
            }
            sleep(::tokio::time::Duration::from_millis(self.nanvixd_ready_retry_interval_ms)).await;
        }

        let reason: String = format!(
            "nanvixd did not expose HTTP endpoint after {} attempts (context={})",
            self.nanvixd_ready_attempts_max,
            self.inner.context_label()
        );
        error!("try_wait_ready(): {reason}");
        Err(::anyhow::anyhow!(reason))
    }

    ///
    /// # Description
    ///
    /// Returns the IPv4:port endpoint exposed by this Nanvix Daemon handle.
    ///
    /// # Return Value
    ///
    /// Returns the socket address formatted as `addr:port`.
    ///
    fn http_address(&self) -> String {
        format!("{}:{}", self.ipv4_addr, self.port_num)
    }
}

//==================================================================================================
// Nanvix Daemon HTTP Arguments
//==================================================================================================

///
/// Bundles the file handles and runtime parameters required when spawning the Nanvix Daemon in
/// HTTP mode.
///
pub struct NanvixdHttpArgs {
    /// File handle that captures Nanvix Daemon stdout.
    stdout_file_handle: File,
    /// File handle that captures Nanvix Daemon stderr.
    stderr_file_handle: File,
    /// IPv4 address exposed by the daemon instance.
    ipv4_addr: String,
    /// TCP port bound by the daemon instance.
    port_num: u16,
    /// Directory where Nanvix Daemon components should emit logs.
    log_directory: PathBuf,
    /// Command-line arguments passed directly to nanvixd.
    extra_nanvixd_args: Vec<String>,
}

impl NanvixdHttpArgs {
    ///
    /// # Description
    ///
    /// Builds the log file and runtime bundle needed to launch the Nanvix Daemon in HTTP mode.
    ///
    /// # Parameters
    ///
    /// - `log_files`: Tuple containing stdout and stderr log file paths.
    /// - `http_endpoint`: Tuple containing the IPv4 address and TCP port for the HTTP interface.
    /// - `log_directory`: Path where component logs should be persisted.
    /// - `extra_nanvixd_args`: Command-line arguments passed directly to nanvixd.
    ///
    /// # Return Value
    ///
    /// Returns a ready-to-use argument bundle when both log files can be created; returns an error
    /// when the stdout or stderr log file cannot be opened.
    ///
    pub fn new(
        log_files: (&Path, &Path),
        http_endpoint: (&str, u16),
        log_directory: &Path,
        extra_nanvixd_args: &[String],
    ) -> Result<Self> {
        let (stdout_file_path, stderr_file_path): (&Path, &Path) = log_files;
        let (ipv4_addr, port_num): (&str, u16) = http_endpoint;
        let stdout_file_handle: File = match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(stdout_file_path)
        {
            Err(error) => {
                let reason: String = format!(
                    "failed to open nanvixd stdout log file (path={}, error={error})",
                    stdout_file_path.display()
                );
                error!("new(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
            Ok(file) => file,
        };

        let stderr_file_handle: File = match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(stderr_file_path)
        {
            Err(error) => {
                let reason: String = format!(
                    "failed to open nanvixd stderr log file (path={}, error={error})",
                    stderr_file_path.display()
                );
                error!("new(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
            Ok(file) => file,
        };

        Ok(Self {
            stdout_file_handle,
            stderr_file_handle,
            ipv4_addr: ipv4_addr.to_string(),
            port_num,
            log_directory: log_directory.to_path_buf(),
            extra_nanvixd_args: extra_nanvixd_args.to_vec(),
        })
    }

    ///
    /// # Description
    ///
    /// Retrieves the stdout log handle used by the Nanvix Daemon when running in HTTP mode.
    ///
    /// # Return Value
    ///
    /// Returns a reference to the stdout file handle.
    ///
    fn stdout_file_handle(&self) -> &File {
        &self.stdout_file_handle
    }

    ///
    /// # Description
    ///
    /// Retrieves the stderr log handle used by the Nanvix Daemon when running in HTTP mode.
    ///
    /// # Return Value
    ///
    /// Returns a reference to the stderr file handle.
    ///
    fn stderr_file_handle(&self) -> &File {
        &self.stderr_file_handle
    }

    ///
    /// # Description
    ///
    /// Returns the TCP port bound by the Nanvix Daemon HTTP endpoint.
    ///
    /// # Return Value
    ///
    /// Returns the TCP port number configured for the daemon.
    ///
    fn port_num(&self) -> u16 {
        self.port_num
    }

    ///
    /// # Description
    ///
    /// Returns the IPv4 address bound by the Nanvix Daemon HTTP endpoint.
    ///
    /// # Return Value
    ///
    /// Returns the IPv4 string configured for the daemon.
    ///
    fn ipv4_addr(&self) -> &str {
        self.ipv4_addr.as_str()
    }

    ///
    /// # Description
    ///
    /// Returns the directory where the Nanvix Daemon should persist component logs.
    ///
    /// # Return Value
    ///
    /// Returns the component log directory as a path reference.
    ///
    fn log_directory(&self) -> &Path {
        self.log_directory.as_path()
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
