// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Host-side driver for the standalone `warm-start-socket` benchmark.
//!
//! The guest runs a TCP echo server (`socket-echo-rust-nostd`) bound to a well-known loopback port.
//! In standalone mode, `networkd` backs the guest socket with a real host-namespace socket, so the
//! host client can exchange stream data with it at `127.0.0.1:<port>`. Each measured request is a
//! length-prefixed payload written over one established TCP connection.

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
use ::log::warn;
use ::std::{
    collections::HashMap,
    net::{
        Ipv4Addr,
        SocketAddr,
        SocketAddrV4,
    },
    time::{
        Duration,
        Instant,
    },
};
use ::tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::TcpStream,
    time::{
        sleep,
        timeout,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Loopback port on which the guest echo server listens. Must match `DEFAULT_ECHO_PORT` in the
/// `socket-echo-rust-nostd` guest application.
pub(crate) const GUEST_ECHO_PORT: u16 = 34254;

/// Payload sizes swept when no explicit `-payload-size` is provided.
const MESSAGE_SIZES: [(&str, usize); 5] = [
    ("32B", 32),
    ("1KiB", 1024),
    ("4KiB", 4 * 1024),
    ("8KiB", 8 * 1024),
    ("16KiB", 16 * 1024),
];

/// Byte value used to fill echo payloads.
const PAYLOAD_FILL_BYTE: u8 = 7;

/// Maximum time to wait for the guest echo server to become reachable before giving up.
const READINESS_TIMEOUT_SECS: u64 = 30;

/// Per-probe receive timeout while waiting for the guest to become reachable.
const READINESS_PROBE_TIMEOUT_MS: u64 = 200;

/// Sleep between readiness probes.
const READINESS_RETRY_SLEEP_MS: u64 = 20;

/// Maximum time to wait for a single echo round trip once the guest is ready.
const ECHO_ROUND_TRIP_TIMEOUT_SECS: u64 = 1;

/// Largest payload accepted by the benchmark protocol.
const MAX_PAYLOAD_SIZE: usize = 64 * 1024;

/// Number of bytes in the host-to-guest length prefix.
const LENGTH_PREFIX_SIZE: usize = 8;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Formats a human-readable label for a payload size.
fn format_message_size(size: usize) -> String {
    if size >= 1024 && size.is_multiple_of(1024) {
        format!("{}KiB", size / 1024)
    } else {
        format!("{size}B")
    }
}

/// Builds the list of `(label, payload)` pairs to benchmark, honoring an optional size override.
fn socket_payloads(payload_size_override: Option<usize>) -> Result<Vec<(String, Vec<u8>)>> {
    let sizes: Vec<(String, usize)> = match payload_size_override {
        Some(size) => vec![(format_message_size(size), size)],
        None => MESSAGE_SIZES
            .iter()
            .map(|(label, size)| ((*label).to_string(), *size))
            .collect(),
    };

    for (_, size) in &sizes {
        if *size == 0 {
            anyhow::bail!("payload size must be positive for warm-start-socket");
        }
        if *size > MAX_PAYLOAD_SIZE {
            anyhow::bail!(
                "payload size must be at most {MAX_PAYLOAD_SIZE} bytes for warm-start-socket"
            );
        }
    }

    Ok(sizes
        .into_iter()
        .map(|(label, size)| (label, vec![PAYLOAD_FILL_BYTE; size]))
        .collect())
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Benchmark {
    /// Drives the TCP echo client against a guest that is already running and whose echo server is
    /// (or will shortly be) bound to [`GUEST_ECHO_PORT`] on loopback.
    ///
    /// The caller is responsible for booting and tearing down the VM. Round-trip latency is
    /// measured entirely over the real standalone `networkd` path.
    pub(crate) async fn run_socket_echo_client(&self) -> Result<()> {
        let payloads: Vec<(String, Vec<u8>)> = socket_payloads(self.payload_size_override)?;

        let total_iterations: u64 = u64::try_from(
            self.iterations
                .checked_mul(payloads.len())
                .ok_or_else(|| anyhow::anyhow!("total iteration count overflows usize"))?,
        )
        .map_err(|e| anyhow::anyhow!("iteration count exceeds u64: {e}"))?;
        let pb: ProgressBar = ProgressBar::new(total_iterations);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .map_err(|e| anyhow::anyhow!("error creating progress bar: {e}"))?
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        let guest_addr: SocketAddr =
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, GUEST_ECHO_PORT));
        let mut stream: TcpStream = self.connect_to_guest(guest_addr).await?;

        let max_payload_size: usize = payloads
            .iter()
            .map(|(_, payload)| payload.len())
            .max()
            .ok_or_else(|| anyhow::anyhow!("no payload sizes configured for warm-start-socket"))?;
        let mut buffer: Vec<u8> = vec![0u8; max_payload_size];

        let mut latencies: HashMap<String, Vec<u128>> = HashMap::new();
        for (label, payload) in &payloads {
            // Warm each payload size independently so first-use buffer growth is not timed.
            echo_round_trip(&mut stream, payload, &mut buffer).await?;
            sleep(Duration::from_millis(WARMUP_SLEEP_DURATION)).await;

            let mut samples: Vec<u128> = Vec::with_capacity(self.iterations);
            for _ in 0..self.iterations {
                let start: Instant = Instant::now();
                echo_round_trip(&mut stream, payload, &mut buffer).await?;
                samples.push(start.elapsed().as_micros());

                sleep(Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
                pb.inc(1);
            }
            latencies.insert(label.clone(), samples);
        }

        pb.finish();

        println!("Size:\tp50\tp95\tp99 [us]");
        for (label, _) in payloads.iter() {
            if let Some(samples) = latencies.get_mut(label) {
                if samples.is_empty() {
                    warn!("No latencies recorded for {label}");
                    continue;
                }

                samples.sort();
                let len: usize = samples.len();
                let p50: u128 = samples[((len as f64 * 0.5) as usize).min(len - 1)];
                let p95: u128 = samples[((len as f64 * 0.95) as usize).min(len - 1)];
                let p99: u128 = samples[((len as f64 * 0.99) as usize).min(len - 1)];
                println!("{label}:\t{p50}\t{p95}\t{p99}");
            } else {
                warn!("No latencies recorded for {label}");
            }
        }

        Ok(())
    }

    /// Polls the guest until its TCP listener accepts a connection or the readiness deadline
    /// elapses.
    ///
    /// Until the guest has booted, bound its socket, and started listening, connection attempts may
    /// fail, so all per-probe errors are treated as "not ready yet" and retried.
    async fn connect_to_guest(&self, guest_addr: SocketAddr) -> Result<TcpStream> {
        let deadline: Instant = Instant::now() + Duration::from_secs(READINESS_TIMEOUT_SECS);

        while Instant::now() < deadline {
            match timeout(
                Duration::from_millis(READINESS_PROBE_TIMEOUT_MS),
                TcpStream::connect(guest_addr),
            )
            .await
            {
                Ok(Ok(stream)) => {
                    stream.set_nodelay(true)?;
                    return Ok(stream);
                },
                // Connection error or timeout: keep waiting.
                _ => {
                    sleep(Duration::from_millis(READINESS_RETRY_SLEEP_MS)).await;
                },
            }
        }

        anyhow::bail!(
            "guest echo server did not become reachable on {guest_addr} within \
             {READINESS_TIMEOUT_SECS}s"
        )
    }
}

/// Performs one timed echo round trip, validating that the guest echoed the exact payload.
async fn echo_round_trip(stream: &mut TcpStream, payload: &[u8], buffer: &mut [u8]) -> Result<()> {
    match timeout(Duration::from_secs(ECHO_ROUND_TRIP_TIMEOUT_SECS), async {
        let payload_size: u64 = u64::try_from(payload.len())
            .map_err(|e| anyhow::anyhow!("payload size exceeds u64: {e}"))?;
        let length_prefix: [u8; LENGTH_PREFIX_SIZE] = payload_size.to_be_bytes();

        stream.write_all(&length_prefix).await?;
        stream.write_all(payload).await?;
        stream.read_exact(&mut buffer[..payload.len()]).await?;

        Ok::<(), anyhow::Error>(())
    })
    .await
    {
        Ok(result) => result?,
        Err(_) => anyhow::bail!(
            "timed out waiting {ECHO_ROUND_TRIP_TIMEOUT_SECS}s for {}-byte echo round trip",
            payload.len()
        ),
    }

    if buffer[..payload.len()] != *payload {
        anyhow::bail!("echoed payload mismatch ({} bytes)", payload.len());
    }

    Ok(())
}
