// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Standalone gateway benchmarks.
//!
//! This module keeps one `nanvixd` process alive and drives User VMs through its HTTP control API
//! and returned gateway endpoint. Warm-start measurements reuse one VM, while cold-start
//! measurements repeatedly create and terminate VMs under the persistent daemon.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    CLEANUP_SLEEP_DURATION,
    WARMUP_SLEEP_DURATION,
};
use crate::benchmark::Benchmark;
use ::anyhow::Result;
use ::indicatif::{
    ProgressBar,
    ProgressStyle,
};
use ::log::{
    debug,
    warn,
};
use ::nanvix::http::message::{
    ErrorResponse,
    HTTP_HEADER_MESSAGE_TYPE,
    Kill,
    KillResponse,
    MessageType,
    New,
    NewResponse,
};
#[cfg(unix)]
use ::nanvix::syscomm::{
    ReadExact,
    SocketStream,
    SocketType,
    UnboundSocket,
    WriteAll,
};
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
use ::std::{
    collections::HashMap,
    net::{
        Ipv4Addr,
        SocketAddr,
        TcpListener,
    },
    process::Stdio,
    time::{
        Duration,
        Instant,
    },
};
use ::tokio::{
    process::{
        Child,
        Command,
    },
    time::{
        sleep,
        timeout,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Payload sizes used by the gateway sweep when no explicit size is requested.
const MESSAGE_SIZES: [(&str, usize); 7] = [
    ("32B", 32),
    ("64B", 64),
    ("128B", 128),
    ("256B", 256),
    ("512B", 512),
    ("1KiB", 1024),
    ("4KiB", 4 * 1024),
];

/// Largest request accepted by the echo benchmark application.
const MAX_PAYLOAD_SIZE: usize = 64 * 1024;

/// Maximum time to wait for one gateway echo, including the initial VM startup.
const GATEWAY_IO_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum time to wait for the daemon HTTP endpoint and gateway endpoint.
const ENDPOINT_READY_TIMEOUT: Duration = Duration::from_secs(30);

/// Delay between daemon and gateway endpoint connection attempts.
const ENDPOINT_RETRY_DELAY: Duration = Duration::from_millis(10);

/// Maximum time to wait for one daemon readiness response.
const READY_PROBE_TIMEOUT: Duration = Duration::from_millis(200);

/// Maximum time to wait for one HTTP control request.
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Maximum time to wait for `nanvixd` after requesting process termination.
const NANVIXD_TEARDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Byte used to fill benchmark payloads.
const PAYLOAD_FILL_BYTE: u8 = 7;

/// Payload used to confirm that a newly spawned User VM is ready.
const COLD_START_PAYLOAD: &[u8] = b"hello";

//==================================================================================================
// Structures
//==================================================================================================

/// Persistent standalone daemon used to create and terminate benchmark User VMs.
struct NanvixdSession {
    /// Running `nanvixd` process.
    child: Child,
    /// HTTP endpoint used for NEW and KILL requests.
    request_url: String,
    /// HTTP client configured without connection pooling.
    client: Client,
    /// Monotonic application identifier used by NEW requests.
    next_app_id: usize,
}

/// User VM identifier and connected gateway stream returned by `nanvixd`.
struct GatewaySession {
    /// NEW response containing the User VM identifier.
    response: NewResponse,
    /// Platform-specific gateway connection.
    stream: GatewayStream,
}

/// Platform-specific connection to the standalone gateway endpoint.
enum GatewayStream {
    /// Unix-domain gateway stream.
    #[cfg(unix)]
    Socket(SocketStream),
    /// Windows named-pipe gateway stream.
    #[cfg(windows)]
    Pipe(::tokio::net::windows::named_pipe::NamedPipeClient),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl NanvixdSession {
    /// Spawns one standalone daemon and waits for its HTTP readiness endpoint.
    async fn spawn(benchmark: &Benchmark) -> Result<Self> {
        let nanvixd_path = benchmark.standalone_nanvixd_path();
        if !nanvixd_path.exists() {
            anyhow::bail!("nanvixd binary not found at {}", nanvixd_path.display());
        }

        let listener: TcpListener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let address: SocketAddr = listener.local_addr()?;
        drop(listener);

        let client: Client = Client::builder()
            .pool_idle_timeout(Duration::ZERO)
            .pool_max_idle_per_host(0)
            .timeout(HTTP_REQUEST_TIMEOUT)
            .build()?;
        let request_url: String = format!("http://{address}");

        let mut command: Command = Command::new(&nanvixd_path);
        command
            .arg(::nanvixd::args::Args::OPT_HTTP_SOCKADDR)
            .arg(address.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .current_dir(&benchmark.workspace_root)
            .kill_on_drop(true);

        let child: Child = command.spawn()?;
        let mut session: Self = Self {
            child,
            request_url,
            client,
            next_app_id: 0,
        };
        if let Err(error) = session.wait_ready().await {
            let shutdown_result: Result<()> = session.shutdown().await;
            return match shutdown_result {
                Ok(()) => Err(error),
                Err(shutdown_error) => Err(anyhow::anyhow!(
                    "{error}; additionally failed to stop nanvixd: {shutdown_error}"
                )),
            };
        }

        Ok(session)
    }

    /// Waits until the daemon answers `GET /ready` or exits early.
    async fn wait_ready(&mut self) -> Result<()> {
        let deadline: Instant = Instant::now() + ENDPOINT_READY_TIMEOUT;
        let ready_url: String = format!("{}/ready", self.request_url);
        loop {
            if let Some(status) = self.child.try_wait()? {
                anyhow::bail!("nanvixd exited before becoming ready (status={status})");
            }

            if let Ok(Ok(response)) =
                timeout(READY_PROBE_TIMEOUT, self.client.get(&ready_url).send()).await
                && response.status().is_success()
            {
                return Ok(());
            }
            if Instant::now() >= deadline {
                anyhow::bail!(
                    "nanvixd did not become ready within {} seconds",
                    ENDPOINT_READY_TIMEOUT.as_secs()
                );
            }
            sleep(ENDPOINT_RETRY_DELAY).await;
        }
    }

    /// Creates a User VM through HTTP and connects to its gateway endpoint.
    async fn spawn_vm(&mut self, benchmark: &Benchmark) -> Result<GatewaySession> {
        let program: String = benchmark.flavour.get_program(&benchmark.workspace_root);
        if !::std::path::Path::new(&program).exists() {
            anyhow::bail!("benchmark program not found at {program}");
        }

        let app_id: usize = self.next_app_id;
        self.next_app_id = self.next_app_id.saturating_add(1);
        let message: New = New {
            tenant_id: "nanvix-bench".to_string(),
            app_name: format!("gateway-{app_id}"),
            program,
            program_args: String::new(),
        };
        let response: ::reqwest::Response = self
            .client
            .post(&self.request_url)
            .headers(message_headers(MessageType::New)?)
            .json(&message)
            .send()
            .await?;
        let status: StatusCode = response.status();
        if !status.is_success() {
            anyhow::bail!(
                "nanvixd rejected NEW request: {}",
                response_error(response, status).await
            );
        }
        let response: NewResponse = response.json().await?;

        match GatewayStream::connect(&response.gateway_sockaddr).await {
            Ok(stream) => Ok(GatewaySession { response, stream }),
            Err(error) => {
                let kill_result: Result<i32> = self.kill_vm(response.user_vm_id).await;
                match kill_result {
                    Ok(_) => Err(error),
                    Err(kill_error) => Err(anyhow::anyhow!(
                        "{error}; additionally failed to terminate User VM: {kill_error}"
                    )),
                }
            },
        }
    }

    /// Closes one gateway input stream and waits for its User VM to exit.
    async fn terminate_vm(&self, mut session: GatewaySession) -> Result<()> {
        let shutdown_result: Result<()> = session
            .stream
            .shutdown_write()
            .await
            .map_err(::anyhow::Error::from);
        let kill_result: Result<()> =
            self.kill_vm(session.response.user_vm_id)
                .await
                .and_then(|exit_code| {
                    if exit_code == 0 {
                        Ok(())
                    } else {
                        anyhow::bail!("gateway User VM exited with status {exit_code}")
                    }
                });

        combine_results(shutdown_result, kill_result)
    }

    /// Sends a KILL request and returns the User VM exit code.
    async fn kill_vm(&self, user_vm_id: ::user_vm_api::UserVmIdentifier) -> Result<i32> {
        let message: Kill = Kill { user_vm_id };
        let response: ::reqwest::Response = self
            .client
            .post(&self.request_url)
            .headers(message_headers(MessageType::Kill)?)
            .json(&message)
            .send()
            .await?;
        let status: StatusCode = response.status();
        if !status.is_success() {
            anyhow::bail!(
                "nanvixd rejected KILL request: {}",
                response_error(response, status).await
            );
        }
        let response: KillResponse = response.json().await?;

        Ok(response.exit_code)
    }

    /// Terminates the persistent daemon process.
    async fn shutdown(mut self) -> Result<()> {
        if let Some(status) = self.child.try_wait()? {
            if !status.success() {
                anyhow::bail!("nanvixd exited unexpectedly (status={status})");
            }
            return Ok(());
        }

        self.child.start_kill()?;
        timeout(NANVIXD_TEARDOWN_TIMEOUT, self.child.wait())
            .await
            .map_err(|_| {
                anyhow::anyhow!(
                    "timed out waiting {} seconds for nanvixd to exit",
                    NANVIXD_TEARDOWN_TIMEOUT.as_secs()
                )
            })??;

        Ok(())
    }
}

impl GatewaySession {
    /// Sends one payload through the gateway and verifies its echoed response.
    async fn round_trip(&mut self, payload: &[u8]) -> Result<()> {
        match timeout(GATEWAY_IO_TIMEOUT, async {
            self.stream.write_all(payload).await?;
            let mut response: Vec<u8> = vec![0u8; payload.len()];
            self.stream.read_exact(&mut response).await?;
            if response != payload {
                anyhow::bail!("gateway echoed payload does not match the request");
            }
            Ok(())
        })
        .await
        {
            Ok(result) => result,
            Err(_) => anyhow::bail!(
                "gateway echo timed out after {} seconds",
                GATEWAY_IO_TIMEOUT.as_secs()
            ),
        }
    }
}

impl GatewayStream {
    /// Connects to a Unix-domain or named-pipe gateway with bounded retries.
    async fn connect(address: &str) -> Result<Self> {
        let deadline: Instant = Instant::now() + ENDPOINT_READY_TIMEOUT;
        loop {
            #[cfg(unix)]
            let result: ::std::io::Result<Self> = UnboundSocket::new(SocketType::Unix)
                .connect(address)
                .await
                .map(Self::Socket);
            #[cfg(windows)]
            let result: ::std::io::Result<Self> =
                ::tokio::net::windows::named_pipe::ClientOptions::new()
                    .open(address)
                    .map(Self::Pipe);

            match result {
                Ok(stream) => return Ok(stream),
                Err(error) if Instant::now() < deadline => {
                    debug!("gateway connection failed, retrying (error={error})");
                    sleep(ENDPOINT_RETRY_DELAY).await;
                },
                Err(error) => {
                    anyhow::bail!(
                        "failed to connect to gateway {address} within {} seconds: {error}",
                        ENDPOINT_READY_TIMEOUT.as_secs()
                    );
                },
            }
        }
    }

    /// Writes one complete gateway input record.
    async fn write_all(&mut self, payload: &[u8]) -> ::std::io::Result<()> {
        match self {
            #[cfg(unix)]
            Self::Socket(stream) => stream.write_all(payload).await,
            #[cfg(windows)]
            Self::Pipe(pipe) => {
                use ::tokio::io::AsyncWriteExt;
                let payload_len: u32 = u32::try_from(payload.len()).map_err(|_| {
                    ::std::io::Error::new(
                        ::std::io::ErrorKind::InvalidInput,
                        "gateway payload exceeds u32 length",
                    )
                })?;
                pipe.write_all(&payload_len.to_le_bytes()).await?;
                pipe.write_all(payload).await
            },
        }
    }

    /// Reads exactly one expected response payload.
    async fn read_exact(&mut self, response: &mut [u8]) -> ::std::io::Result<()> {
        match self {
            #[cfg(unix)]
            Self::Socket(stream) => stream.read_exact(response).await.map(|_| ()),
            #[cfg(windows)]
            Self::Pipe(pipe) => {
                use ::tokio::io::AsyncReadExt;
                pipe.read_exact(response).await.map(|_| ())
            },
        }
    }

    /// Signals end-of-input while preserving the output direction.
    async fn shutdown_write(&mut self) -> ::std::io::Result<()> {
        match self {
            #[cfg(unix)]
            Self::Socket(stream) => stream.shutdown_write().await,
            #[cfg(windows)]
            Self::Pipe(pipe) => {
                use ::tokio::io::AsyncWriteExt;
                pipe.write_all(&0u32.to_le_bytes()).await?;
                pipe.flush().await
            },
        }
    }
}

impl Benchmark {
    /// Measures fresh User VM startup through the first standalone gateway echo.
    pub async fn run_cold_start_uvm(&mut self) -> Result<()> {
        let progress: ProgressBar = ProgressBar::new(
            self.iterations
                .try_into()
                .map_err(|error| anyhow::anyhow!("iteration count exceeds u64: {error}"))?,
        );
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .map_err(|error| anyhow::anyhow!("error creating progress bar: {error}"))?
                .progress_chars("#>-"),
        );
        progress.set_message("Benchmark progress:");

        let mut nanvixd: NanvixdSession = NanvixdSession::spawn(self).await?;
        let run_result: Result<()> = async {
            let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
            for _ in 0..self.iterations {
                let start: Instant = Instant::now();
                let mut session: GatewaySession = nanvixd.spawn_vm(self).await?;
                let echo_result: Result<()> = session.round_trip(COLD_START_PAYLOAD).await;
                let latency: u128 = start.elapsed().as_micros();
                let terminate_result: Result<()> = nanvixd.terminate_vm(session).await;
                combine_results(echo_result, terminate_result)?;

                latencies.push(latency);
                progress.inc(1);
                sleep(Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
            }

            progress.finish();
            println!("First req: {} us", latencies[0]);
            latencies.sort();
            println!("p50: {} us", latencies[(self.iterations as f64 * 0.50) as usize]);
            println!("p95: {} us", latencies[(self.iterations as f64 * 0.95) as usize]);
            println!("p99: {} us", latencies[(self.iterations as f64 * 0.99) as usize]);

            Ok(())
        }
        .await;
        let shutdown_result: Result<()> = nanvixd.shutdown().await;

        combine_results(run_result, shutdown_result)
    }

    /// Runs steady-state round-trip measurements through the standalone gateway.
    pub async fn run_warm_start_gateway(&mut self) -> Result<()> {
        let payloads: Vec<(String, Vec<u8>)> = gateway_payloads(self.payload_size_override)?;
        let total_iterations: u64 = u64::try_from(
            self.iterations
                .checked_mul(payloads.len())
                .ok_or_else(|| anyhow::anyhow!("total iteration count overflows usize"))?,
        )
        .map_err(|error| anyhow::anyhow!("iteration count exceeds u64: {error}"))?;

        let progress: ProgressBar = ProgressBar::new(total_iterations);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .map_err(|error| anyhow::anyhow!("error creating progress bar: {error}"))?
                .progress_chars("#>-"),
        );
        progress.set_message("Benchmark progress:");

        let mut nanvixd: NanvixdSession = NanvixdSession::spawn(self).await?;
        let run_result: Result<()> = async {
            let mut session: GatewaySession = nanvixd.spawn_vm(self).await?;
            let measurement_result: Result<()> = async {
                let mut latencies: HashMap<String, Vec<u128>> = HashMap::new();
                for (label, payload) in &payloads {
                    session.round_trip(payload).await?;
                    sleep(Duration::from_millis(WARMUP_SLEEP_DURATION)).await;

                    let mut samples: Vec<u128> = Vec::with_capacity(self.iterations);
                    for _ in 0..self.iterations {
                        let start: Instant = Instant::now();
                        session.round_trip(payload).await?;
                        samples.push(start.elapsed().as_micros());

                        sleep(Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
                        progress.inc(1);
                    }
                    latencies.insert(label.clone(), samples);
                }

                progress.finish();
                println!("Size:\tp50\tp95\tp99 [us]");
                for (label, _) in &payloads {
                    if let Some(samples) = latencies.get_mut(label) {
                        samples.sort();
                        let len: usize = samples.len();
                        let p50: u128 = samples[((len as f64 * 0.50) as usize).min(len - 1)];
                        let p95: u128 = samples[((len as f64 * 0.95) as usize).min(len - 1)];
                        let p99: u128 = samples[((len as f64 * 0.99) as usize).min(len - 1)];
                        println!("{label}:\t{p50}\t{p95}\t{p99}");
                    } else {
                        warn!("no latencies recorded for {label}");
                    }
                }

                Ok(())
            }
            .await;
            let terminate_result: Result<()> = nanvixd.terminate_vm(session).await;

            combine_results(measurement_result, terminate_result)
        }
        .await;
        let shutdown_result: Result<()> = nanvixd.shutdown().await;

        combine_results(run_result, shutdown_result)
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Builds the payload list for a fixed-size run or the default gateway sweep.
fn gateway_payloads(payload_size_override: Option<usize>) -> Result<Vec<(String, Vec<u8>)>> {
    let sizes: Vec<(String, usize)> = match payload_size_override {
        Some(size) => vec![(format_message_size(size), size)],
        None => MESSAGE_SIZES
            .iter()
            .map(|(label, size)| ((*label).to_string(), *size))
            .collect(),
    };

    for (_, size) in &sizes {
        if *size == 0 {
            anyhow::bail!("payload size must be positive for warm-start-gateway");
        }
        if *size > MAX_PAYLOAD_SIZE {
            anyhow::bail!(
                "payload size must be at most {MAX_PAYLOAD_SIZE} bytes for warm-start-gateway"
            );
        }
    }

    Ok(sizes
        .into_iter()
        .map(|(label, size)| (label, vec![PAYLOAD_FILL_BYTE; size]))
        .collect())
}

/// Formats a byte count for percentile output.
fn format_message_size(size: usize) -> String {
    if size >= 1024 && size.is_multiple_of(1024) {
        format!("{}KiB", size / 1024)
    } else {
        format!("{size}B")
    }
}

/// Builds HTTP headers for a standalone control request.
fn message_headers(message_type: MessageType) -> Result<HeaderMap> {
    let mut headers: HeaderMap = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(CONNECTION, HeaderValue::from_static("close"));
    headers.insert(HTTP_HEADER_MESSAGE_TYPE, HeaderValue::from_str(&message_type.to_string())?);

    Ok(headers)
}

/// Formats an unsuccessful HTTP response using its structured payload when available.
async fn response_error(response: ::reqwest::Response, status: StatusCode) -> String {
    match response.json::<ErrorResponse>().await {
        Ok(error) => format!("status={status}, code={}, message={}", error.code, error.message),
        Err(decode_error) => {
            format!("status={status}, failed to decode error response: {decode_error}")
        },
    }
}

/// Combines benchmark and shutdown outcomes without losing either error.
fn combine_results(run_result: Result<()>, shutdown_result: Result<()>) -> Result<()> {
    match (run_result, shutdown_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(run_error), Err(cleanup_error)) => Err(anyhow::anyhow!(
            "{run_error}; additionally failed to clean up benchmark resources: {cleanup_error}"
        )),
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_payloads_use_legacy_sweep() {
        let payloads: Vec<(String, Vec<u8>)> =
            gateway_payloads(None).expect("default gateway payloads should be valid");
        let labels: Vec<&str> = payloads.iter().map(|(label, _)| label.as_str()).collect();

        assert_eq!(labels, ["32B", "64B", "128B", "256B", "512B", "1KiB", "4KiB"]);
    }

    #[test]
    fn gateway_payloads_honor_size_override() {
        let payloads: Vec<(String, Vec<u8>)> =
            gateway_payloads(Some(1024)).expect("gateway payload override should be valid");

        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].0, "1KiB");
        assert_eq!(payloads[0].1.len(), 1024);
    }

    #[test]
    fn gateway_payloads_reject_oversized_request() {
        let error: anyhow::Error = gateway_payloads(Some(MAX_PAYLOAD_SIZE + 1))
            .expect_err("oversized gateway payload should fail");

        assert!(error.to_string().contains("at most"), "unexpected error: {error}");
    }
}
