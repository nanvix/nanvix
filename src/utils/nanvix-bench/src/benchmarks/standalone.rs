// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::benchmark::Benchmark;
use ::anyhow::Result;
use ::indicatif::{
    ProgressBar,
    ProgressStyle,
};
use ::log::error;
#[cfg(feature = "profile-time")]
use ::std::collections::HashMap;
use ::std::{
    path::PathBuf,
    time::Instant,
};
use ::tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    process::Command,
    time::Duration,
};

#[cfg(feature = "profile-time")]
use ::nanvix::uservm::perf::PERF_TIMINGS_PREFIX;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Payload sent to the echo program during each cold-start iteration.
///
const ECHO_PAYLOAD: &[u8] = b"hello\n";

///
/// # Description
///
/// Timeout (in seconds) for a single cold-start iteration. If the iteration exceeds this
/// duration, it is considered failed.
///
const COLD_START_TIMEOUT_SECS: u64 = 120;

/// Timeout for reading perf timing data from stderr during command execution.
#[cfg(feature = "profile-time")]
const STDERR_READ_TIMEOUT: Duration = Duration::from_millis(500);

/// Phase names in display order for the timing breakdown table.
#[cfg(feature = "profile-time")]
const PHASE_NAMES: &[&str] = &[
    "channel_setup_us",
    "partition_create_us",
    "vmem_create_us",
    "vcpu_create_us",
    "kernel_load_us",
    "initrd_load_us",
    "ramfs_load_us",
    "vcpu_reset_us",
    "thread_spawn_us",
    "guest_exec_us",
    "exit_handling_us",
    "total_us",
];

/// Human-readable labels for each phase, matching [`PHASE_NAMES`] order.
#[cfg(feature = "profile-time")]
const PHASE_LABELS: &[&str] = &[
    "channel_setup",
    "partition_create",
    "vmem_create",
    "vcpu_create",
    "kernel_load",
    "initrd_load",
    "ramfs_load",
    "vcpu_reset",
    "thread_spawn",
    "guest_exec",
    "exit_handling",
    "total",
];

//==================================================================================================
// Implementations
//==================================================================================================

impl Benchmark {
    ///
    /// # Description
    ///
    /// Runs the standalone cold-start benchmark. Each iteration spawns a fresh nanvixd process in
    /// interactive mode with the echo program, writes a payload to stdin, and measures the time
    /// until the echo response arrives on stdout.
    ///
    pub async fn run_cold_start_standalone(&mut self) -> Result<()> {
        let nanvixd_bin: PathBuf = self.standalone_nanvixd_path();
        let program: String = self.flavour.get_program(&self.workspace_root);

        if !nanvixd_bin.exists() {
            let reason: String = format!("nanvixd binary not found at {}", nanvixd_bin.display());
            error!("{reason}");
            anyhow::bail!(reason);
        }

        let pb: ProgressBar = ProgressBar::new(self.iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        #[cfg(feature = "profile-time")]
        let mut phase_samples: HashMap<String, Vec<u64>> = HashMap::new();
        #[cfg(feature = "profile-time")]
        for name in PHASE_NAMES {
            phase_samples.insert((*name).to_string(), Vec::with_capacity(self.iterations));
        }

        for _ in 0..self.iterations {
            #[cfg(feature = "profile-time")]
            let (latency, phase_timings) = ::tokio::time::timeout(
                Duration::from_secs(COLD_START_TIMEOUT_SECS),
                self.run_standalone_iteration(&nanvixd_bin, &program),
            )
            .await
            .map_err(|_| {
                anyhow::anyhow!("cold-start iteration timed out after {COLD_START_TIMEOUT_SECS}s")
            })??;

            #[cfg(not(feature = "profile-time"))]
            let (latency, _phase_timings) = ::tokio::time::timeout(
                Duration::from_secs(COLD_START_TIMEOUT_SECS),
                self.run_standalone_iteration(&nanvixd_bin, &program),
            )
            .await
            .map_err(|_| {
                anyhow::anyhow!("cold-start iteration timed out after {COLD_START_TIMEOUT_SECS}s")
            })??;

            latencies.push(latency);

            // Accumulate per-phase timing samples and forward raw data to stderr.
            #[cfg(feature = "profile-time")]
            if let Some(ref timings) = phase_timings {
                // Re-emit the raw PERF_TIMINGS line so external tools can parse it.
                // Use pb.suspend() to temporarily clear the progress bar before writing,
                // preventing interleaving with the bar's carriage-return redraws on stderr.
                // When stderr is not a tty (redirected to file), suspend() is a no-op and
                // eprintln! writes cleanly.
                pb.suspend(|| {
                    eprintln!(
                        "{}{}",
                        ::nanvix::uservm::perf::PERF_TIMINGS_PREFIX,
                        serde_json::Value::Object(timings.clone())
                    );
                });
                for name in PHASE_NAMES {
                    if let Some(value) = timings.get(*name)
                        && let Some(samples) = phase_samples.get_mut(*name)
                        && let Some(v) = value.as_u64()
                    {
                        samples.push(v);
                    }
                }
            }

            pb.inc(1);
        }

        pb.finish();
        println!("First req: {} us", latencies[0]);
        latencies.sort();
        println!("p50: {} us", latencies[(self.iterations as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(self.iterations as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(self.iterations as f32 * 0.99) as usize]);

        // Print per-phase timing breakdown if any phase data was collected.
        #[cfg(feature = "profile-time")]
        {
            let has_phase_data: bool = phase_samples.values().any(|v| !v.is_empty());
            if has_phase_data {
                println!();
                println!(
                    "{:<22} {:>10} {:>10} {:>10}",
                    "Phase", "p50 (us)", "p95 (us)", "p99 (us)"
                );
                println!("{}", "-".repeat(54));
                for (name, label) in PHASE_NAMES.iter().zip(PHASE_LABELS.iter()) {
                    if let Some(samples) = phase_samples.get_mut(*name) {
                        if samples.is_empty() {
                            continue;
                        }
                        samples.sort();
                        let len: usize = samples.len();
                        let p50: u64 = samples[((len as f32 * 0.5) as usize).min(len - 1)];
                        let p95: u64 = samples[((len as f32 * 0.95) as usize).min(len - 1)];
                        let p99: u64 = samples[((len as f32 * 0.99) as usize).min(len - 1)];
                        println!("{:<22} {:>10} {:>10} {:>10}", label, p50, p95, p99);
                    }
                }
            }
        }

        Ok(())
    }

    /// Runs a single standalone cold-start iteration, returning the latency in microseconds
    /// and an optional JSON map of per-phase timings from the uservm.
    async fn run_standalone_iteration(
        &self,
        nanvixd_bin: &PathBuf,
        program: &str,
    ) -> Result<(u128, Option<serde_json::Map<String, serde_json::Value>>)> {
        let start: Instant = Instant::now();

        let mut cmd: Command = Command::new(nanvixd_bin);
        cmd.arg(::nanvixd::args::Args::OPT_SEPARATOR)
            .arg(program)
            .stdin(::std::process::Stdio::piped())
            .stdout(::std::process::Stdio::piped())
            .current_dir(&self.workspace_root)
            .kill_on_drop(true);

        // Only pipe stderr when profiling is enabled to avoid blocking on a full pipe.
        #[cfg(feature = "profile-time")]
        cmd.stderr(::std::process::Stdio::piped());
        #[cfg(not(feature = "profile-time"))]
        cmd.stderr(::std::process::Stdio::null());

        let mut child: ::tokio::process::Child = cmd.spawn()?;

        // Write payload to stdin and drop it. Closing the pipe delivers EOF to the guest, causing
        // the echo program to exit after processing the payload.
        {
            let mut stdin: ::tokio::process::ChildStdin = child
                .stdin
                .take()
                .ok_or_else(|| anyhow::anyhow!("failed to take nanvixd stdin"))?;
            stdin.write_all(ECHO_PAYLOAD).await?;
        }

        // Read the echo response from stdout. The first data arriving confirms that the VM booted
        // and the echo program processed the payload.
        {
            let mut stdout: ::tokio::process::ChildStdout = child
                .stdout
                .take()
                .ok_or_else(|| anyhow::anyhow!("failed to take nanvixd stdout"))?;
            let mut buf: Vec<u8> = vec![0u8; ECHO_PAYLOAD.len()];
            stdout.read_exact(&mut buf).await?;
        }

        let elapsed: Duration = start.elapsed();

        // Read stderr to capture perf timing data before waiting for exit.
        #[cfg(feature = "profile-time")]
        let phase_timings: Option<serde_json::Map<String, serde_json::Value>> = {
            let mut stderr: ::tokio::process::ChildStderr = match child.stderr.take() {
                Some(stderr) => stderr,
                None => {
                    // Wait for nanvixd to exit cleanly.
                    let _ = child.wait().await;
                    return Ok((elapsed.as_micros(), None));
                },
            };
            let mut stderr_buf: Vec<u8> = Vec::new();
            // Read all available stderr data with a short timeout.
            // NOTE: this assumes LOG_LEVEL=panic so that the child does not flood stderr
            // and block on a full pipe before the benchmark reads it.
            let _ =
                ::tokio::time::timeout(STDERR_READ_TIMEOUT, stderr.read_to_end(&mut stderr_buf))
                    .await;

            parse_perf_timings(&stderr_buf)
        };
        #[cfg(not(feature = "profile-time"))]
        let phase_timings: Option<serde_json::Map<String, serde_json::Value>> = None;

        // Wait for nanvixd to exit cleanly.
        let _ = child.wait().await;

        Ok((elapsed.as_micros(), phase_timings))
    }

    /// Returns the path to the `nanvixd`-compatible binary to benchmark.
    ///
    /// When the `NANVIX_BENCH_NANVIXD` environment variable is set, its value is
    /// used verbatim; this allows the same standalone driver to benchmark an
    /// alternative VMM front-end (e.g. the OpenVMM-based `nanvixd-vmm`) for a
    /// side-by-side comparison against the native `nanvixd.elf` (uservm).
    /// Otherwise it defaults to the platform `nanvixd` binary under `bin/`.
    pub(crate) fn standalone_nanvixd_path(&self) -> PathBuf {
        if let Some(path) = ::std::env::var_os("NANVIX_BENCH_NANVIXD") {
            return PathBuf::from(path);
        }
        let bin_name: &str = if cfg!(windows) {
            "nanvixd.exe"
        } else {
            "nanvixd.elf"
        };
        self.workspace_root.join("bin").join(bin_name)
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Parses a `PERF_TIMINGS:{...}` JSON line from stderr output and returns the deserialized map.
#[cfg(feature = "profile-time")]
fn parse_perf_timings(stderr_bytes: &[u8]) -> Option<serde_json::Map<String, serde_json::Value>> {
    let stderr_str: &str = std::str::from_utf8(stderr_bytes).ok()?;
    for line in stderr_str.lines() {
        if let Some(json_str) = line.strip_prefix(PERF_TIMINGS_PREFIX)
            && let Ok(serde_json::Value::Object(map)) = serde_json::from_str(json_str)
        {
            return Some(map);
        }
    }
    None
}
