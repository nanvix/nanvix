// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::config::RunnerConfig;
use ::anyhow::Result;
use ::log::{
    debug,
    error,
    trace,
};
use ::nanvix::{
    http::message::{
        ErrorResponse,
        HTTP_HEADER_MESSAGE_TYPE,
        Kill,
        KillResponse,
        MessageType,
        New,
        NewResponse,
    },
    sandbox::UserVmIdentifier,
    syscomm::{
        ReadExact,
        SocketStream,
        SocketType,
        WriteAll,
    },
};
// The socket-based gateway transport is only used on Unix; on Windows the standalone gateway is
// exposed as a named pipe (see `GatewayStream::connect`).
#[cfg(unix)]
use ::nanvix::syscomm::UnboundSocket;
use ::reqwest::{
    Client,
    StatusCode,
    header::{
        CONNECTION,
        CONTENT_TYPE,
        HeaderMap,
        HeaderValue,
    },
};
use ::tokio::{
    task::block_in_place,
    time::{
        Duration,
        Instant,
        sleep,
    },
};

//==================================================================================================
// User VM Handle
//==================================================================================================

///
/// # Description
///
/// Handle to a User VM started through the Nanvix Daemon REST API.
///
pub struct UserVm {
    /// Fully qualified Nanvix Daemon endpoint used when talking to the control plane.
    request_url: String,
    /// Identifier assigned to this User VM by the Nanvix Daemon.
    user_vm_id: UserVmIdentifier,
    /// Socket stream wired to the User VM gateway for I/O.
    gateway_stream: GatewayStream,
    /// Milliseconds to wait after shutting down a User VM.
    cleanup_uservm_sleep_duration_ms: u64,
    /// Indicates whether the User VM has been explicitly terminated.
    terminated: bool,
}

impl UserVm {
    ///
    /// # Description
    ///
    /// Spawns a User VM by issuing a REST request to the Nanvix Daemon and wiring up the gateway
    /// socket returned by the service.
    ///
    /// # Parameters
    ///
    /// - `config`: Runtime configuration that provides the Nanvix Daemon endpoint and cleanup
    ///   timings.
    /// - `uservm_args`: Headers, workload metadata, and flags that describe the User VM to launch.
    ///
    /// # Return Value
    ///
    /// Returns a handle bound to the newly created User VM when the REST call and gateway
    /// connection succeed; returns an error when the request or socket setup fails.
    ///
    pub async fn spawn(config: &RunnerConfig, uservm_args: &UserVmArgs) -> Result<Self> {
        let http_endpoint: String = config.http_endpoint();
        let request_url: String = format!("http://{http_endpoint}");
        let client: Client = Self::build_control_plane_client()?;
        trace!("spawn(): http_endpoint={}", http_endpoint);

        let payload: New = New {
            tenant_id: uservm_args.tenant_id.clone(),
            app_name: uservm_args.app_name.clone(),
            program: uservm_args.program_path.clone(),
            program_args: uservm_args.combined_program_args(),
        };

        let mut request_headers: HeaderMap = uservm_args.headers();
        request_headers.insert(CONNECTION, HeaderValue::from_static("close"));

        let http_response: ::reqwest::Response = match client
            .post(request_url.as_str())
            .headers(request_headers)
            .json(&payload)
            .send()
            .await
        {
            Err(error) => {
                let reason: String = format!(
                    "failed to send user VM spawn request (url={}, error={error})",
                    request_url
                );
                error!("spawn(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
            Ok(resp) => resp,
        };

        let status: StatusCode = http_response.status();
        let response: NewResponse = if status.is_success() {
            match http_response.json::<NewResponse>().await {
                Err(error) => {
                    let reason: String = format!(
                        "failed to decode user VM spawn response (url={}, error={error})",
                        request_url
                    );
                    error!("spawn(): {reason}");
                    return Err(::anyhow::anyhow!(reason));
                },
                Ok(parsed) => parsed,
            }
        } else {
            let details: String = Self::format_error_details(http_response, status).await;
            let reason: String =
                format!("nanvixd rejected user VM spawn (url={}, details={details})", request_url);
            error!("spawn(): {reason}");
            return Err(::anyhow::anyhow!(reason));
        };

        debug!("spawn(): uservm id={}, gateway={}", response.user_vm_id, response.gateway_sockaddr);

        let gateway_socktype: SocketType = SocketType::Unix;

        let gateway_stream: GatewayStream =
            Self::connect_to_gateway(config, response.gateway_sockaddr.as_str(), gateway_socktype)
                .await?;

        debug!("spawn(): connected to uservm gateway stream");

        Ok(Self {
            request_url,
            user_vm_id: response.user_vm_id,
            gateway_stream,
            cleanup_uservm_sleep_duration_ms: config.cleanup_uservm_sleep_duration_ms,
            terminated: false,
        })
    }

    ///
    /// # Description
    ///
    /// Attempts to connect to the gateway socket created by nanvixd using a bounded retry loop
    /// with exponential backoff. This prevents the harness from hanging indefinitely when the
    /// gateway never becomes reachable.
    ///
    /// # Parameters
    ///
    /// - `address`: Socket address returned by nanvixd for the uservm gateway.
    /// - `socket_type`: Indicates whether the gateway is exposed via UNIX or TCP sockets.
    ///
    /// # Return Value
    ///
    /// Returns a connected `GatewayStream` when the gateway becomes reachable before the timeout;
    /// returns an error when the retry budget is exhausted.
    ///
    async fn connect_to_gateway(
        config: &RunnerConfig,
        address: &str,
        socket_type: SocketType,
    ) -> Result<GatewayStream> {
        let deadline: Duration = Duration::from_millis(config.gateway_connect_timeout_ms);
        let start: Instant = Instant::now();
        let mut attempts: usize = 0;
        let mut backoff_ms: u64 = config.gateway_connect_initial_backoff_ms;
        let max_attempts: usize = config.gateway_connect_max_attempts;
        let max_backoff_ms: u64 = config.gateway_connect_max_backoff_ms;

        loop {
            attempts = attempts.saturating_add(1);
            match GatewayStream::connect(socket_type, address).await {
                Ok(stream) => return Ok(stream),
                Err(error) => {
                    if error.kind() == ::std::io::ErrorKind::Unsupported {
                        let reason: String = format!(
                            "unsupported gateway transport for address {address} (error={error})"
                        );
                        error!("connect_to_gateway(): {reason}");
                        return Err(::anyhow::anyhow!(reason));
                    }
                    let elapsed: Duration = start.elapsed();
                    debug!(
                        "connect_to_gateway(): attempt {} failed (addr={}, elapsed_ms={}, \
                         error={error}), retrying after {} ms",
                        attempts,
                        address,
                        elapsed.as_millis(),
                        backoff_ms
                    );

                    if attempts >= max_attempts || elapsed >= deadline {
                        let reason: String = format!(
                            "failed to connect to gateway socket (addr={}, attempts={}, \
                             elapsed_ms={}, last_error={error})",
                            address,
                            attempts,
                            elapsed.as_millis()
                        );
                        error!("connect_to_gateway(): {reason}");
                        return Err(::anyhow::anyhow!(reason));
                    }

                    sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms.saturating_mul(2)).min(max_backoff_ms);
                },
            }
        }
    }

    ///
    /// # Description
    ///
    /// Builds an HTTP client with connection pooling disabled so each control-plane request uses
    /// a fresh TCP session. This avoids reusing stale keep-alive sockets between the `New` and
    /// `Kill` requests issued by the test harness.
    ///
    /// # Return Value
    ///
    /// Returns a configured HTTP client on success; returns an error if the builder fails.
    ///
    fn build_control_plane_client() -> Result<Client> {
        Client::builder()
            .pool_idle_timeout(Duration::from_secs(0))
            .pool_max_idle_per_host(0)
            .build()
            .map_err(|error| {
                let reason: String =
                    format!("failed to build control-plane client without pooling (error={error})");
                error!("build_control_plane_client(): {reason}");
                ::anyhow::anyhow!(reason)
            })
    }

    ///
    /// # Description
    ///
    /// Provides mutable access to the gateway stream used for User VM I/O.
    ///
    /// # Return Value
    ///
    /// Returns a mutable reference to the socket stream connected to the User VM gateway.
    ///
    pub fn gateway_stream(&mut self) -> &mut GatewayStream {
        &mut self.gateway_stream
    }

    ///
    /// # Description
    ///
    /// Computes how long callers should wait after shutting down this User VM using the timing
    /// configuration supplied by the test runner.
    ///
    /// # Return Value
    ///
    /// Returns the duration callers should wait before launching another User VM.
    ///
    fn cleanup_delay(&self) -> Duration {
        Duration::from_millis(self.cleanup_uservm_sleep_duration_ms)
    }

    ///
    /// # Description
    ///
    /// Attempts to decode a structured error payload returned by nanvixd for better diagnostics.
    ///
    /// # Parameters
    ///
    /// - `response`: Raw HTTP response returned by nanvixd.
    /// - `status`: HTTP status code associated with the response.
    ///
    /// # Return Value
    ///
    /// Returns a string that either embeds the decoded error payload or contains a fallback
    /// message when decoding fails.
    ///
    async fn format_error_details(response: ::reqwest::Response, status: StatusCode) -> String {
        match response.json::<ErrorResponse>().await {
            Ok(payload) => {
                format!("status={}, code={}, message={}", status, payload.code, payload.message)
            },
            Err(error) => {
                format!("status={}, failed to decode error payload (error={error})", status)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Terminates the User VM by issuing a REST request to the Nanvix Daemon and waiting for the
    /// cleanup delay to elapse.
    ///
    /// # Parameters
    ///
    /// - `request_url`: Endpoint used to reach the Nanvix Daemon.
    /// - `user_vm_id`: Identifier of the User VM that should be terminated.
    ///
    /// # Return Value
    ///
    /// Returns the exit code reported by the User VM on success; returns an error if the request
    /// or response handling fails.
    ///
    async fn kill(&self, request_url: String, user_vm_id: UserVmIdentifier) -> Result<i32> {
        trace!("kill(): user_vm_id={user_vm_id}");

        let client: Client = Self::build_control_plane_client()?;

        let mut headers: HeaderMap = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let message_type_value: HeaderValue =
            match HeaderValue::from_str(MessageType::Kill.to_string().as_str()) {
                Ok(value) => value,
                Err(error) => {
                    let reason: String =
                        format!("failed to build header value for MessageType::Kill ({error})");
                    error!("kill(): {reason}");
                    return Err(::anyhow::anyhow!(reason));
                },
            };
        headers.insert(HTTP_HEADER_MESSAGE_TYPE, message_type_value);
        headers.insert(CONNECTION, HeaderValue::from_static("close"));

        let kill_msg: Kill = Kill { user_vm_id };

        let http_response: ::reqwest::Response = match client
            .post(request_url.as_str())
            .headers(headers)
            .json(&kill_msg)
            .send()
            .await
        {
            Err(error) => {
                let reason: String = format!(
                    "failed to send user VM kill request (url={}, error={error})",
                    request_url
                );
                error!("kill(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
            Ok(resp) => resp,
        };

        let status: StatusCode = http_response.status();
        let response: KillResponse = if status.is_success() {
            match http_response.json::<KillResponse>().await {
                Err(error) => {
                    let reason: String = format!(
                        "failed to decode user VM kill response (url={}, error={error})",
                        request_url
                    );
                    error!("kill(): {reason}");
                    return Err(::anyhow::anyhow!(reason));
                },
                Ok(parsed) => parsed,
            }
        } else {
            let details: String = Self::format_error_details(http_response, status).await;
            let reason: String = format!(
                "nanvixd rejected user VM kill (user_vm_id={}, details={details})",
                user_vm_id
            );
            error!("kill(): {reason}");
            return Err(::anyhow::anyhow!(reason));
        };

        let exit_code: i32 = response.exit_code;
        if exit_code != 0 {
            debug!(
                "kill(): nanvixd reported non-zero exit code (user_vm_id={}, exit_code={})",
                user_vm_id, exit_code
            );
        } else {
            debug!("kill(): uservm {} terminated", user_vm_id);
        }

        sleep(self.cleanup_delay()).await;

        Ok(exit_code)
    }

    ///
    /// # Description
    ///
    /// Explicitly terminates the User VM and returns the exit code reported by the workload.
    ///
    /// This method should be called when the caller needs to validate the exit code. After
    /// calling this method, the User VM will not be terminated again when dropped.
    ///
    /// # Thread Safety
    ///
    /// This method requires exclusive access (`&mut self`), which guarantees that `terminate()`
    /// and `drop()` cannot race. The `terminated` flag is only accessed through mutable references.
    ///
    /// # Return Value
    ///
    /// Returns the exit code reported by the User VM on success; returns an error if the
    /// termination request fails.
    ///
    pub async fn terminate(&mut self) -> Result<i32> {
        let request_url: String = self.request_url.clone();
        let user_vm_id: UserVmIdentifier = self.user_vm_id;
        let exit_code: i32 = self.kill(request_url, user_vm_id).await?;
        self.terminated = true;
        Ok(exit_code)
    }
}

impl Drop for UserVm {
    ///
    /// # Description
    ///
    /// Ensures the User VM is terminated when this handle goes out of scope by synchronously
    /// driving the asynchronous `kill()` helper. Skips termination if `terminate()` was already
    /// called.
    ///
    /// # Return Value
    ///
    /// Returns `()`; logs errors when termination cannot be completed.
    ///
    fn drop(&mut self) {
        trace!("drop(): user_vm_id={}", self.user_vm_id);

        // Skip termination if already explicitly terminated.
        if self.terminated {
            trace!("drop(): user_vm_id={} already terminated, skipping", self.user_vm_id);
            return;
        }

        if let Ok(handle) = ::tokio::runtime::Handle::try_current() {
            let request_url: String = self.request_url.clone();
            let user_vm_id: UserVmIdentifier = self.user_vm_id;

            let kill_result: Result<i32> =
                block_in_place(|| handle.block_on(self.kill(request_url, user_vm_id)));

            match kill_result {
                Ok(exit_code) if exit_code != 0 => {
                    debug!(
                        "drop(): user VM terminated with non-zero exit code (user_vm_id={}, \
                         exit_code={exit_code})",
                        self.user_vm_id
                    );
                },
                Ok(_) => {},
                Err(error) => {
                    error!(
                        "drop(): failed to terminate user VM (user_vm_id={}, error={error})",
                        self.user_vm_id
                    );
                },
            }
        } else {
            match ::tokio::runtime::Runtime::new() {
                Ok(runtime) => match runtime
                    .block_on(self.kill(self.request_url.clone(), self.user_vm_id))
                {
                    Ok(exit_code) if exit_code != 0 => {
                        debug!(
                            "drop(): user VM terminated with non-zero exit code (user_vm_id={}, \
                             exit_code={exit_code})",
                            self.user_vm_id
                        );
                    },
                    Ok(_) => {},
                    Err(error) => {
                        error!(
                            "drop(): failed to terminate user VM (user_vm_id={}, error={error})",
                            self.user_vm_id
                        );
                    },
                },
                Err(error) => {
                    error!(
                        "drop(): failed to build runtime for user VM termination (user_vm_id={}, \
                         error={error})",
                        self.user_vm_id
                    );
                },
            }
        }
    }
}

//==================================================================================================
// Gateway Stream
//==================================================================================================

///
/// # Description
///
/// Transport-agnostic handle to the User VM gateway endpoint exposed by nanvixd.
///
/// The standalone gateway is a single point at which the test harness exchanges guest stdio.
/// Its underlying transport is platform-dependent:
///
/// - **Unix**: a Unix-domain socket, wrapped in [`SocketStream`].
/// - **Windows**: a named pipe (`\\.\pipe\...`), since Unix-domain sockets are unavailable.
///   Windows only supports standalone deployments. Because
///   named pipes have no half-close primitive, the input (stdin) direction is framed to emulate
///   one: each record is a little-endian `u32` length followed by that many payload bytes, and a
///   zero-length record signals EOF. The output direction stays a raw byte stream.
///
pub(crate) enum GatewayStream {
    /// Gateway exposed via a `syscomm` socket stream (TCP, or a Unix-domain socket on Unix).
    /// Only constructed on Unix; on Windows the gateway is always a named pipe.
    #[cfg_attr(windows, allow(dead_code))]
    Socket(SocketStream),
    /// Gateway exposed via a Windows named pipe (used by the standalone deployment).
    #[cfg(windows)]
    Pipe(::tokio::net::windows::named_pipe::NamedPipeClient),
}

impl GatewayStream {
    ///
    /// # Description
    ///
    /// Establishes a single connection to the gateway endpoint, selecting the transport that
    /// matches the requested socket type and host platform.
    ///
    /// # Parameters
    ///
    /// - `socket_type`: Transport requested by the caller (`Unix` for standalone deployments).
    /// - `address`: Gateway endpoint address returned by nanvixd.
    ///
    /// # Return Value
    ///
    /// Returns a connected gateway stream on success; returns an I/O error on failure so the
    /// caller's retry loop can decide whether to try again.
    ///
    pub(crate) async fn connect(socket_type: SocketType, address: &str) -> ::std::io::Result<Self> {
        // On Windows the standalone gateway is exposed as a named pipe rather than a
        // Unix-domain socket, which the platform does not provide. TCP gateways are
        // not supported on Windows, so reject them up front rather than falling through to the
        // socket path and surfacing confusing connection retries/timeouts.
        #[cfg(windows)]
        {
            match socket_type {
                SocketType::Unix => {
                    let client: ::tokio::net::windows::named_pipe::NamedPipeClient =
                        ::tokio::net::windows::named_pipe::ClientOptions::new().open(address)?;
                    Ok(GatewayStream::Pipe(client))
                },
                SocketType::Tcp => Err(::std::io::Error::new(
                    ::std::io::ErrorKind::Unsupported,
                    "TCP gateways are not supported on Windows",
                )),
            }
        }

        #[cfg(unix)]
        {
            let stream: SocketStream = UnboundSocket::new(socket_type).connect(address).await?;
            Ok(GatewayStream::Socket(stream))
        }
    }

    ///
    /// # Description
    ///
    /// Writes the entire buffer to the gateway endpoint.
    ///
    pub(crate) async fn write_all(&mut self, buf: &[u8]) -> ::std::io::Result<()> {
        match self {
            GatewayStream::Socket(stream) => stream.write_all(buf).await,
            #[cfg(windows)]
            GatewayStream::Pipe(pipe) => {
                use ::tokio::io::AsyncWriteExt;
                // The Windows gateway emulates the Unix half-close with a framed input
                // direction: each record is a little-endian `u32` length followed by that many
                // payload bytes (see `shutdown_write` for the matching zero-length EOF record).
                // Skip empty writes so a zero-length payload is never mistaken for the EOF record.
                if buf.is_empty() {
                    return Ok(());
                }
                let payload_len: u32 = u32::try_from(buf.len()).map_err(|_| {
                    ::std::io::Error::new(
                        ::std::io::ErrorKind::InvalidInput,
                        "gateway input record exceeds u32 length",
                    )
                })?;
                pipe.write_all(&payload_len.to_le_bytes()).await?;
                pipe.write_all(buf).await
            },
        }
    }

    ///
    /// # Description
    ///
    /// Signals end-of-input to the gateway endpoint.
    ///
    /// On Unix this half-closes the write direction so the peer observes EOF on the guest's stdin.
    ///
    /// Windows named pipes have no half-close primitive. The gateway therefore emulates one with a
    /// framed input direction: this writes a zero-length EOF record so the daemon-side bridge
    /// closes the guest's stdin while the pipe stays open for reading the guest's output.
    ///
    pub(crate) async fn shutdown_write(&mut self) -> ::std::io::Result<()> {
        match self {
            GatewayStream::Socket(stream) => stream.shutdown_write().await,
            #[cfg(windows)]
            GatewayStream::Pipe(pipe) => {
                use ::tokio::io::AsyncWriteExt;
                // Zero-length record = in-band EOF marker. Flush so the bridge observes it
                // promptly; the pipe stays open so guest output can still be read.
                pipe.write_all(&0u32.to_le_bytes()).await?;
                pipe.flush().await
            },
        }
    }

    ///
    /// # Description
    ///
    /// Reads exactly enough bytes to fill the provided buffer from the gateway endpoint.
    ///
    pub(crate) async fn read_exact(&mut self, buf: &mut [u8]) -> ::std::io::Result<usize> {
        match self {
            GatewayStream::Socket(stream) => stream.read_exact(buf).await,
            #[cfg(windows)]
            GatewayStream::Pipe(pipe) => {
                use ::tokio::io::AsyncReadExt;
                pipe.read_exact(buf).await?;
                Ok(buf.len())
            },
        }
    }
}

//==================================================================================================
// User VM Arguments
//==================================================================================================

///
/// # Description
///
/// Arguments required to request a new User VM deployment from the Nanvix Daemon.
pub struct UserVmArgs {
    /// HTTP headers forwarded to the Nanvix Daemon REST endpoint.
    headers: HeaderMap,
    /// Tenant identifier forwarded to the Nanvix Daemon.
    tenant_id: String,
    /// Application workload name used to identify the sandbox.
    app_name: String,
    /// Binary path that should execute inside the User VM.
    program_path: String,
    /// Optional command-line arguments forwarded to the workload.
    program_args: Option<String>,
    /// Optional environment variables forwarded to the workload (combined into program_args
    /// using the documented `<args>;<env>` format before sending to nanvixd).
    program_env: Option<String>,
}

impl UserVmArgs {
    ///
    /// # Description
    ///
    /// Creates and configures the HTTP headers and workload metadata required to request a new
    /// User VM deployment via the Nanvix Daemon REST API.
    ///
    /// # Parameters
    ///
    /// - `tenant_id`: Identifier used to isolate the sandbox resources.
    /// - `app_name`: Human-readable application workload name.
    /// - `program_path`: Absolute path to the executable launched inside the User VM.
    /// - `program_args`: Optional command-line arguments forwarded to the executable.
    /// - `program_env`: Optional environment variables forwarded to the executable.
    ///
    /// # Return Value
    ///
    /// Returns a fully prepared argument bundle with headers and metadata when header
    /// construction succeeds; returns an error if the message-type header value cannot be built.
    ///
    pub fn new(
        tenant_id: &str,
        app_name: &str,
        program_path: &str,
        program_args: Option<&str>,
        program_env: Option<&str>,
    ) -> Result<Self> {
        let mut headers: HeaderMap = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let message_type_value: HeaderValue =
            match HeaderValue::from_str(MessageType::New.to_string().as_str()) {
                Ok(value) => value,
                Err(error) => {
                    let reason: String =
                        format!("failed to build header value for MessageType::New ({error})");
                    error!("new(): {reason}");
                    return Err(::anyhow::anyhow!(reason));
                },
            };
        headers.insert(HTTP_HEADER_MESSAGE_TYPE, message_type_value);

        Ok(Self {
            headers,
            tenant_id: tenant_id.to_string(),
            app_name: app_name.to_string(),
            program_path: program_path.to_string(),
            program_args: program_args.map(|value| value.to_string()),
            program_env: program_env.map(|value| value.to_string()),
        })
    }

    ///
    /// # Description
    ///
    /// Returns a fresh copy of the HTTP headers prepared for the Nanvix Daemon request.
    ///
    /// # Return Value
    ///
    /// Returns a header map containing the content type and message type markers needed to spawn
    /// the User VM.
    ///
    pub fn headers(&self) -> HeaderMap {
        self.headers.clone()
    }

    ///
    /// # Description
    ///
    /// Builds the combined `program_args` string using the documented `<args>;<env>` format.
    ///
    /// When environment variables are present, they are appended after a `;` separator so that
    /// the kernel's `split_cmdline()` can split them. When only one of args or env is present,
    /// the appropriate prefix or suffix is used.
    ///
    /// # Return Value
    ///
    /// Returns the combined string ready to be sent as the `program_args` field in the HTTP
    /// `New` message.
    ///
    fn combined_program_args(&self) -> String {
        crate::executor::combine_args_env(self.program_args.as_deref(), self.program_env.as_deref())
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::combine_args_env;

    /// Helper that builds a `UserVmArgs` with the given program_args and program_env, then
    /// calls `combined_program_args()`.
    fn combine(args: Option<&str>, env: Option<&str>) -> String {
        let uva: UserVmArgs =
            UserVmArgs::new("t", "a", "p", args, env).expect("UserVmArgs::new failed");
        uva.combined_program_args()
    }

    #[test]
    fn combined_no_args_no_env() {
        assert_eq!(combine(None, None), "");
    }

    #[test]
    fn combined_args_only() {
        assert_eq!(combine(Some("arg1 arg2"), None), "arg1 arg2");
    }

    #[test]
    fn combined_env_only() {
        assert_eq!(combine(None, Some("VAR=x")), ";VAR=x");
    }

    #[test]
    fn combined_args_and_env() {
        assert_eq!(combine(Some("arg1"), Some("VAR=x")), "arg1;VAR=x");
    }

    #[test]
    fn combined_escapes_semicolons_in_args() {
        // A literal `;` in args must be escaped to `\;` so split_cmdline() treats it as data.
        assert_eq!(combine(Some("a;b"), Some("VAR=x")), "a\\;b;VAR=x");
    }

    #[test]
    fn combined_escapes_semicolons_in_env() {
        // A literal `;` in env must be escaped to `\;` so split_cmdline() treats it as data.
        assert_eq!(combine(Some("arg1"), Some("PATH=a;b")), "arg1;PATH=a\\;b");
    }

    #[test]
    fn combined_escapes_semicolons_even_without_env() {
        // Semicolons in args are always escaped so split_cmdline() treats them as data.
        assert_eq!(combine(Some("a;b"), None), "a\\;b");
    }

    #[test]
    fn combined_roundtrip_with_split_cmdline() {
        // Verify the combined string round-trips through the kernel's split_cmdline().
        let combined: String = combine(Some("path/to;file arg2"), Some("FOO=bar BAZ=qux"));
        let mut buf: Vec<u8> = combined.into_bytes();
        let (args, env) = ::cmdline::split_cmdline(&mut buf);
        assert_eq!(args, "path/to;file arg2");
        assert_eq!(env, "FOO=bar BAZ=qux");
    }

    #[test]
    fn combined_roundtrip_semicolon_in_env_value() {
        // Verify that a literal `;` inside an env value survives the round-trip
        // because it is escaped to `\;` and split_cmdline() unescapes it.
        let combined: String = combine(Some("hello"), Some("PATH=a;b"));
        let mut buf: Vec<u8> = combined.into_bytes();
        let (args, env) = ::cmdline::split_cmdline(&mut buf);
        assert_eq!(args, "hello");
        assert_eq!(env, "PATH=a;b");
    }

    #[test]
    fn combined_empty_env_string_treated_as_absent() {
        // `Some("")` must behave the same as `None` — no trailing `;`.
        assert_eq!(combine(Some("arg1"), Some("")), "arg1");
    }

    #[test]
    fn combined_empty_args_string_treated_as_absent() {
        // `Some("")` must behave the same as `None` — no leading `;` when only env is set.
        assert_eq!(combine(Some(""), Some("VAR=x")), ";VAR=x");
    }

    #[test]
    fn combined_program_args_matches_free_function() {
        // The free function should produce identical results to the UserVmArgs method.
        assert_eq!(
            combine_args_env(Some("arg1"), Some("VAR=x")),
            combine(Some("arg1"), Some("VAR=x"))
        );
        assert_eq!(combine_args_env(None, None), combine(None, None));
        assert_eq!(
            combine_args_env(Some("a;b"), Some("PATH=a;b")),
            combine(Some("a;b"), Some("PATH=a;b"))
        );
    }
}
