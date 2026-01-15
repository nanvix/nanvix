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
use ::nanvix::log::{
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
    /// - `nanvixd_args`: File handles and runtime arguments used when spawning the Nanvix Daemon.
    ///
    /// # Return Value
    ///
    /// Returns a handle to the HTTP-enabled Nanvix Daemon on success; returns an error when the
    /// child process cannot be spawned or the readiness checks fail.
    ///
    pub async fn spawn(config: &RunnerConfig, nanvixd_args: &NanvixdHttpArgs) -> Result<Self> {
        let hwloc_file_path: Option<&str> = nanvixd_args.hwloc_file_path();
        let l2_enabled: bool = nanvixd_args.l2();
        let log_directory: &Path = nanvixd_args.log_directory();
        trace!(
            "spawn(): nanvixd_binary_path={}, working_directory={}, toolchain_path={}, mode=http, \
             hwloc_file_path={:?}, l2={}",
            config.nanvixd_binary_path,
            config.working_directory,
            config.toolchain_path,
            hwloc_file_path,
            l2_enabled,
        );

        let port_num: u16 = nanvixd_args.port_num();
        let http_address: String = format!("{}:{}", nanvixd_args.ipv4_addr(), port_num);

        let mut command: Command =
            Nanvixd::build_base_command(config, hwloc_file_path, l2_enabled, log_directory);

        let stdout_file: File = nanvixd_args
            .stdout_file_handle()
            .try_clone()
            .map_err(|error| {
                let reason: String =
                    format!("failed to clone nanvixd stdout log file (error={error})");
                error!("spawn(): {reason}");
                ::anyhow::anyhow!(reason)
            })?;

        let stderr_file: File = nanvixd_args
            .stderr_file_handle()
            .try_clone()
            .map_err(|error| {
                let reason: String =
                    format!("failed to clone nanvixd stderr log file (error={error})");
                error!("spawn(): {reason}");
                ::anyhow::anyhow!(reason)
            })?;

        command.stdin(Stdio::null());
        command.stdout(Stdio::from(stdout_file));
        command.stderr(Stdio::from(stderr_file));
        command.arg(::nanvixd::args::Args::OPT_NETNS_POOL_SIZE);
        command.arg(nanvixd_args.netns_pool_size().to_string());
        command.arg(::nanvixd::args::Args::OPT_HTTP_SOCKADDR);
        command.arg(http_address.as_str());

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
                    let ipv4_addr: String = nanvixd_args.ipv4_addr().to_string();
                    let cleanup_guard: EnvironmentCleanupGuard = EnvironmentCleanupGuard::new(
                        l2_enabled,
                        Some(port_num),
                        PathBuf::from(config.tmp_directory.as_str()),
                        config.tcp_cleanup_max_wait_seconds,
                        config.tcp_cleanup_poll_interval_seconds,
                    );

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
    /// Optional hwloc topology file forwarded to the Nanvix Daemon.
    hwloc_file_path: Option<String>,
    /// Indicates whether L2 deployment mode is enabled.
    l2: bool,
    /// IPv4 address exposed by the daemon instance.
    ipv4_addr: String,
    /// TCP port bound by the daemon instance.
    port_num: u16,
    /// Netns pool prefill size forwarded to the Nanvix Daemon.
    netns_pool_size: usize,
    /// Directory where Nanvix Daemon components should emit logs.
    log_directory: PathBuf,
}

impl NanvixdHttpArgs {
    ///
    /// # Description
    ///
    /// Builds the log file and runtime bundle needed to launch the Nanvix Daemon in HTTP mode.
    ///
    /// # Parameters
    ///
    /// - `stdout_file_path`: Path to the file that captures Nanvix Daemon standard output.
    /// - `stderr_file_path`: Path to the file that captures Nanvix Daemon standard error.
    /// - `ipv4_addr`: IPv4 address where the Nanvix Daemon should expose its HTTP interface.
    /// - `port_num`: TCP port used by the Nanvix Daemon HTTP interface.
    /// - `hwloc_file_path`: Optional hwloc topology file passed to the Nanvix Daemon.
    /// - `l2`: Flag indicating whether the Nanvix Daemon should enable L2 deployment mode.
    /// - `netns_pool_size`: Netns pool prefill size forwarded to the Nanvix Daemon.
    /// - `log_directory`: Path where component logs should be persisted.
    ///
    /// # Return Value
    ///
    /// Returns a ready-to-use argument bundle when both log files can be created; returns an error
    /// when the stdout or stderr log file cannot be opened.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stdout_file_path: &Path,
        stderr_file_path: &Path,
        ipv4_addr: &str,
        port_num: u16,
        hwloc_file_path: Option<String>,
        l2: bool,
        netns_pool_size: usize,
        log_directory: &Path,
    ) -> Result<Self> {
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
            hwloc_file_path,
            l2,
            ipv4_addr: ipv4_addr.to_string(),
            port_num,
            netns_pool_size,
            log_directory: log_directory.to_path_buf(),
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
    /// Retrieves the optional hwloc topology file path passed to the Nanvix Daemon.
    ///
    /// # Return Value
    ///
    /// Returns the optional hwloc path as a string slice.
    ///
    fn hwloc_file_path(&self) -> Option<&str> {
        self.hwloc_file_path.as_deref()
    }

    ///
    /// # Description
    ///
    /// Reports whether L2 deployment mode is enabled for the Nanvix Daemon.
    ///
    /// # Return Value
    ///
    /// Returns `true` when L2 mode is enabled; otherwise returns `false`.
    ///
    fn l2(&self) -> bool {
        self.l2
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
    /// Returns the netns pool prefill size forwarded to the Nanvix Daemon.
    ///
    /// # Return Value
    ///
    /// Returns the netns pool size.
    ///
    fn netns_pool_size(&self) -> usize {
        self.netns_pool_size
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
}
