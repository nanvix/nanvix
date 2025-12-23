// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::RunnerConfig,
    warn_with_policy,
};
use ::anyhow::Result;
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
    log::{
        debug,
        error,
        trace,
    },
    sandbox::UserVmIdentifier,
    syscomm::{
        SocketStream,
        SocketType,
        UnboundSocket,
    },
};
use ::reqwest::{
    Client,
    StatusCode,
    header::{
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
    /// HTTP client reused for Nanvix Daemon requests.
    client: Client,
    /// Fully qualified Nanvix Daemon endpoint used when talking to the control plane.
    request_url: String,
    /// Identifier assigned to this User VM by the Nanvix Daemon.
    user_vm_id: UserVmIdentifier,
    /// Socket stream wired to the User VM gateway for I/O.
    gateway_stream: SocketStream,
    /// Milliseconds to wait after shutting down a User VM when L2 is disabled.
    cleanup_uservm_sleep_duration_ms: u64,
    /// Milliseconds to wait after shutting down a User VM when L2 is enabled.
    cleanup_l2_uservm_sleep_duration_ms: u64,
    /// Indicates whether this User VM handle operates in L2 mode.
    l2_enabled: bool,
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
        let client: Client = Client::new();
        let http_endpoint: String = config.http_endpoint();
        let request_url: String = format!("http://{http_endpoint}");
        let l2_enabled: bool = uservm_args.l2_enabled();
        trace!("spawn(): http_endpoint={}, l2_enabled={}", http_endpoint, l2_enabled);

        let payload: New = New {
            tenant_id: uservm_args.tenant_id.clone(),
            app_name: uservm_args.app_name.clone(),
            program: uservm_args.program_path.clone(),
            program_args: uservm_args.program_args.clone().unwrap_or_default(),
        };

        let http_response: ::reqwest::Response = match client
            .post(request_url.as_str())
            .headers(uservm_args.headers())
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

        let gateway_socktype: SocketType = if l2_enabled {
            SocketType::Tcp
        } else {
            SocketType::Unix
        };

        let gateway_stream: SocketStream =
            Self::connect_to_gateway(config, response.gateway_sockaddr.as_str(), gateway_socktype)
                .await?;

        debug!("spawn(): connected to uservm gateway stream");

        Ok(Self {
            client,
            request_url,
            user_vm_id: response.user_vm_id,
            gateway_stream,
            cleanup_uservm_sleep_duration_ms: config.cleanup_uservm_sleep_duration_ms,
            cleanup_l2_uservm_sleep_duration_ms: config.cleanup_l2_uservm_sleep_duration_ms,
            l2_enabled,
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
    /// Returns a connected `SocketStream` when the gateway becomes reachable before the timeout;
    /// returns an error when the retry budget is exhausted.
    ///
    async fn connect_to_gateway(
        config: &RunnerConfig,
        address: &str,
        socket_type: SocketType,
    ) -> Result<SocketStream> {
        let deadline: Duration = Duration::from_millis(config.gateway_connect_timeout_ms);
        let start: Instant = Instant::now();
        let mut attempts: usize = 0;
        let mut backoff_ms: u64 = config.gateway_connect_initial_backoff_ms;
        let max_attempts: usize = config.gateway_connect_max_attempts;
        let max_backoff_ms: u64 = config.gateway_connect_max_backoff_ms;

        loop {
            attempts = attempts.saturating_add(1);
            let unbound_socket: UnboundSocket = UnboundSocket::new(socket_type);
            match unbound_socket.connect(address).await {
                Ok(stream) => return Ok(stream),
                Err(error) => {
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
    /// Provides mutable access to the gateway stream used for User VM I/O.
    ///
    /// # Return Value
    ///
    /// Returns a mutable reference to the socket stream connected to the User VM gateway.
    ///
    pub fn gateway_stream(&mut self) -> &mut SocketStream {
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
        let sleep_ms: u64 = if self.l2_enabled {
            self.cleanup_l2_uservm_sleep_duration_ms
        } else {
            self.cleanup_uservm_sleep_duration_ms
        };

        Duration::from_millis(sleep_ms)
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
    /// - `client`: HTTP client reused for the shutdown request.
    /// - `request_url`: Endpoint used to reach the Nanvix Daemon.
    /// - `user_vm_id`: Identifier of the User VM that should be terminated.
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` once the Nanvix Daemon confirms the User VM termination response; returns
    /// an error if the request or response handling fails.
    ///
    async fn kill(
        &self,
        client: Client,
        request_url: String,
        user_vm_id: UserVmIdentifier,
    ) -> Result<()> {
        trace!("kill(): user_vm_id={user_vm_id}");

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

        if response.exit_code != 0 {
            warn_with_policy!(
                "kill(): nanvixd reported non-zero exit code (user_vm_id={}, exit_code={})",
                user_vm_id,
                response.exit_code
            );
        } else {
            debug!("kill(): uservm {} terminated", user_vm_id);
        }

        sleep(self.cleanup_delay()).await;

        Ok(())
    }
}

impl Drop for UserVm {
    ///
    /// # Description
    ///
    /// Ensures the User VM is terminated when this handle goes out of scope by synchronously
    /// driving the asynchronous `kill()` helper.
    ///
    /// # Return Value
    ///
    /// Returns `()`; logs errors when termination cannot be completed.
    ///
    fn drop(&mut self) {
        trace!("drop(): user_vm_id={}", self.user_vm_id);

        if let Ok(handle) = ::tokio::runtime::Handle::try_current() {
            let client: Client = self.client.clone();
            let request_url: String = self.request_url.clone();
            let user_vm_id: UserVmIdentifier = self.user_vm_id;

            let kill_result: Result<()> =
                block_in_place(|| handle.block_on(self.kill(client, request_url, user_vm_id)));

            if let Err(error) = kill_result {
                error!(
                    "drop(): failed to terminate user VM (user_vm_id={}, error={error})",
                    self.user_vm_id
                );
            }
        } else {
            match ::tokio::runtime::Runtime::new() {
                Ok(runtime) => match runtime.block_on(self.kill(
                    self.client.clone(),
                    self.request_url.clone(),
                    self.user_vm_id,
                )) {
                    Ok(()) => {},
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
    /// Indicates whether the Nanvix Daemon should provision L2 networking.
    l2_enabled: bool,
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
    /// - `l2_enabled`: Flag indicating whether the request should enable L2 networking mode.
    ///
    /// # Return Value
    ///
    /// Returns a fully prepared argument bundle with headers, metadata, and L2 flag when header
    /// construction succeeds; returns an error if the message-type header value cannot be built.
    ///
    pub fn new(
        tenant_id: &str,
        app_name: &str,
        program_path: &str,
        program_args: Option<&str>,
        l2_enabled: bool,
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
            l2_enabled,
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
    /// Reports whether the caller asked for L2 networking mode when building these arguments.
    ///
    /// # Return Value
    ///
    /// Returns `true` if L2 networking mode should be enabled for the User VM; otherwise returns
    /// `false`.
    ///
    pub fn l2_enabled(&self) -> bool {
        self.l2_enabled
    }
}
