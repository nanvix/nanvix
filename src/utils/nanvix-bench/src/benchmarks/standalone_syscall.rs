// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Standalone deployment of the `syscall-bench` benchmark.
//!
//! Spawns one `nanvixd` process in standalone mode with the `syscall-bench-nostd` guest program
//! and drives it over the console. A system call costs far less than a console round trip, so the
//! host times batches of calls: each sample asks the guest to issue a call `N` times back to back,
//! subtracts the round trip of an empty request that is timed right next to the batch, and divides
//! the difference by `N`. The batch size is calibrated per call, so that each batch runs for at
//! least [`TARGET_BATCH_DURATION`] and the jitter of the console round trip stays small next to it.

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
use ::std::{
    path::{
        Path,
        PathBuf,
    },
    process::{
        ExitStatus,
        Stdio,
    },
    time::Instant,
};
use ::syscall_bench_common::{
    ACK_OK,
    REQUEST_SIZE,
    SyscallOp,
    encode_request,
};
use ::tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    process::{
        Child,
        ChildStdin,
        ChildStdout,
        Command,
    },
    time::{
        Duration,
        timeout,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// System calls measured by the benchmark, grouped by the component that serves them.
///
/// Each group is reported in a latency table of its own, titled with the name of the group.
const SYSCALL_GROUPS: &[(&str, &[(&str, SyscallOp)])] = &[
    (
        "Kernel",
        &[
            ("getppid", SyscallOp::GetPpid),
            ("clock_gettime", SyscallOp::ClockGettime),
            ("sched_yield", SyscallOp::SchedYield),
        ],
    ),
    (
        "Daemons",
        &[
            ("getpgrp", SyscallOp::GetPgrp),
            ("fstat", SyscallOp::Fstat),
            ("stat", SyscallOp::Stat),
            ("read", SyscallOp::Read),
            ("write", SyscallOp::Write),
            ("open+close", SyscallOp::OpenClose),
        ],
    ),
];

/// Minimum time that a calibrated batch takes beyond the console round trip.
const TARGET_BATCH_DURATION: Duration = Duration::from_millis(10);

/// Largest batch size that calibration may pick.
const MAX_BATCH_SIZE: u32 = 1 << 24;

/// Maximum time to wait for the guest to boot and report that it is ready.
const READY_TIMEOUT: Duration = Duration::from_secs(120);

/// Maximum time to wait for the acknowledgement of a request.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum time to wait for `nanvixd` to exit once the guest console is closed.
const TEARDOWN_TIMEOUT: Duration = Duration::from_secs(30);

//==================================================================================================
// Structures
//==================================================================================================

/// Latency samples of one system call.
struct SyscallSamples {
    /// Name of the system call.
    name: &'static str,
    /// Number of calls in each batch.
    batch_size: u32,
    /// Latency of one call in each sample, in nanoseconds.
    latencies: Vec<u128>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Benchmark {
    ///
    /// # Description
    ///
    /// Runs the standalone system call benchmark.
    ///
    /// Boots the guest program once, measures the latency of every system call in
    /// [`SYSCALL_GROUPS`], and prints one percentile latency table per group.
    ///
    pub async fn run_syscall_bench_standalone(&mut self) -> Result<()> {
        let nanvixd_bin: PathBuf = self.standalone_nanvixd_path();
        if !nanvixd_bin.exists() {
            let reason: String = format!("nanvixd binary not found at {}", nanvixd_bin.display());
            error!("{reason}");
            anyhow::bail!(reason);
        }

        let program: String = self.flavour.get_program(&self.workspace_root);
        if !Path::new(&program).exists() {
            let reason: String = format!("benchmark program not found at {program}");
            error!("{reason}");
            anyhow::bail!(reason);
        }

        // Boot the guest. Its stderr is inherited so that boot failures remain visible, and
        // `kill_on_drop` tears the VM down even on an early return.
        let mut child: Child = Command::new(&nanvixd_bin)
            .arg(::nanvixd::args::Args::OPT_SEPARATOR)
            .arg(&program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .current_dir(&self.workspace_root)
            .kill_on_drop(true)
            .spawn()?;
        let mut stdin: ChildStdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to take nanvixd stdin"))?;
        let mut stdout: ChildStdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to take nanvixd stdout"))?;

        let result: Result<Vec<(&str, Vec<SyscallSamples>)>> =
            self.measure_syscalls(&mut stdin, &mut stdout).await;

        // Tear down the VM regardless of the measurement outcome.
        let teardown_result: Result<()> =
            Self::teardown_syscall_bench_nanvixd(child, stdin, stdout).await;

        let groups: Vec<(&str, Vec<SyscallSamples>)> = match (result, teardown_result) {
            (Ok(groups), Ok(())) => groups,
            (Err(error), Ok(())) | (Ok(_), Err(error)) => return Err(error),
            (Err(error), Err(teardown_error)) => {
                anyhow::bail!("{error}; additionally failed to tear down nanvixd: {teardown_error}")
            },
        };

        Self::print_syscall_bench_results(groups);

        Ok(())
    }

    /// Waits for the guest to get ready and measures every system call in [`SYSCALL_GROUPS`].
    async fn measure_syscalls(
        &self,
        stdin: &mut ChildStdin,
        stdout: &mut ChildStdout,
    ) -> Result<Vec<(&'static str, Vec<SyscallSamples>)>> {
        Self::wait_for_syscall_bench_guest(stdout).await?;

        let syscall_count: usize = SYSCALL_GROUPS
            .iter()
            .map(|(_, syscalls)| syscalls.len())
            .sum();
        let pb: ProgressBar = ProgressBar::new((syscall_count * self.iterations) as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")?
                .progress_chars("#>-"),
        );
        pb.set_message("System calls:");

        let mut groups: Vec<(&str, Vec<SyscallSamples>)> = Vec::with_capacity(SYSCALL_GROUPS.len());
        for &(group, syscalls) in SYSCALL_GROUPS {
            let mut samples: Vec<SyscallSamples> = Vec::with_capacity(syscalls.len());
            for &(name, op) in syscalls {
                let batch_size: u32 = Self::calibrate_syscall_batch_size(stdin, stdout, op).await?;

                let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
                for i in 0..self.iterations {
                    // Alternate which request of the pair goes first to cancel ordering bias.
                    let elapsed: Duration =
                        Self::time_syscall_batch(stdin, stdout, op, batch_size, i % 2 == 0).await?;
                    latencies.push(elapsed.as_nanos() / u128::from(batch_size));
                    pb.inc(1);
                }

                samples.push(SyscallSamples {
                    name,
                    batch_size,
                    latencies,
                });
            }
            groups.push((group, samples));
        }
        pb.finish();

        Ok(groups)
    }

    /// Calibrates the number of calls to `op` in a batch.
    ///
    /// Doubles the batch size until a batch takes at least [`TARGET_BATCH_DURATION`] beyond the
    /// console round trip. Each step keeps the shorter of two timings, so that a single delayed
    /// round trip cannot end the calibration early. Calibrating also warms up the call path.
    async fn calibrate_syscall_batch_size(
        stdin: &mut ChildStdin,
        stdout: &mut ChildStdout,
        op: SyscallOp,
    ) -> Result<u32> {
        let mut batch_size: u32 = 1;
        loop {
            let first: Duration =
                Self::time_syscall_batch(stdin, stdout, op, batch_size, true).await?;
            let second: Duration =
                Self::time_syscall_batch(stdin, stdout, op, batch_size, false).await?;
            if first.min(second) >= TARGET_BATCH_DURATION || batch_size >= MAX_BATCH_SIZE {
                return Ok(batch_size);
            }
            batch_size = batch_size.saturating_mul(2).min(MAX_BATCH_SIZE);
        }
    }

    /// Times a batch of `batch_size` calls to `op` and returns the time that it takes beyond the
    /// console round trip.
    ///
    /// The round trip is timed with an empty request that is sent right before the batch when
    /// `noop_first` is set, and right after it otherwise.
    async fn time_syscall_batch(
        stdin: &mut ChildStdin,
        stdout: &mut ChildStdout,
        op: SyscallOp,
        batch_size: u32,
        noop_first: bool,
    ) -> Result<Duration> {
        let (batch, round_trip): (Duration, Duration) = if noop_first {
            let round_trip: Duration =
                Self::send_syscall_request(stdin, stdout, SyscallOp::Noop, batch_size).await?;
            let batch: Duration = Self::send_syscall_request(stdin, stdout, op, batch_size).await?;
            (batch, round_trip)
        } else {
            let batch: Duration = Self::send_syscall_request(stdin, stdout, op, batch_size).await?;
            let round_trip: Duration =
                Self::send_syscall_request(stdin, stdout, SyscallOp::Noop, batch_size).await?;
            (batch, round_trip)
        };

        Ok(batch.saturating_sub(round_trip))
    }

    /// Sends one request to the guest and returns its round-trip time.
    async fn send_syscall_request(
        stdin: &mut ChildStdin,
        stdout: &mut ChildStdout,
        op: SyscallOp,
        count: u32,
    ) -> Result<Duration> {
        let request: [u8; REQUEST_SIZE] = encode_request(op, count);
        let mut ack: [u8; 1] = [0; 1];

        let start: Instant = Instant::now();
        timeout(REQUEST_TIMEOUT, async {
            stdin.write_all(&request).await?;
            stdin.flush().await?;
            stdout.read_exact(&mut ack).await
        })
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "timed out after {}s waiting for {op:?} (batch size {count})",
                REQUEST_TIMEOUT.as_secs()
            )
        })?
        .map_err(|error| anyhow::anyhow!("failed to complete {op:?} request: {error}"))?;
        let elapsed: Duration = start.elapsed();

        if ack[0] != ACK_OK {
            anyhow::bail!("guest failed to run {op:?} (batch size {count})");
        }

        Ok(elapsed)
    }

    /// Waits for the guest to report that it is ready to serve requests.
    async fn wait_for_syscall_bench_guest(stdout: &mut ChildStdout) -> Result<()> {
        let mut ack: [u8; 1] = [0; 1];
        timeout(READY_TIMEOUT, stdout.read_exact(&mut ack))
            .await
            .map_err(|_| anyhow::anyhow!("guest was not ready after {}s", READY_TIMEOUT.as_secs()))?
            .map_err(|error| anyhow::anyhow!("failed to read guest readiness: {error}"))?;

        if ack[0] != ACK_OK {
            anyhow::bail!("guest failed to set up the system call benchmark");
        }

        Ok(())
    }

    /// Closes the guest console, which makes the guest exit, and waits for `nanvixd` to exit.
    async fn teardown_syscall_bench_nanvixd(
        mut child: Child,
        stdin: ChildStdin,
        mut stdout: ChildStdout,
    ) -> Result<()> {
        drop(stdin);

        let status: ExitStatus = timeout(TEARDOWN_TIMEOUT, async {
            // Drain the console so that nanvixd never blocks on a full pipe while exiting.
            let mut output: Vec<u8> = Vec::new();
            stdout.read_to_end(&mut output).await?;
            child.wait().await
        })
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "timed out after {}s waiting for nanvixd to exit",
                TEARDOWN_TIMEOUT.as_secs()
            )
        })?
        .map_err(|error| anyhow::anyhow!("failed to wait for nanvixd to exit: {error}"))?;

        if !status.success() {
            anyhow::bail!("nanvixd exited with {status}");
        }

        Ok(())
    }

    /// Prints the calibrated batch sizes, followed by one latency table per group of system calls.
    fn print_syscall_bench_results(groups: Vec<(&str, Vec<SyscallSamples>)>) {
        println!("\nBatch sizes (calls per sample):");
        for (_, samples) in &groups {
            for sample in samples {
                println!("  {:<20} {:>10}", sample.name, sample.batch_size);
            }
        }

        for (group, samples) in groups {
            let mut data: Vec<(&str, Vec<u128>)> = samples
                .into_iter()
                .map(|sample| (sample.name, sample.latencies))
                .collect();
            super::print_latency_table(group, "ns", &mut data);
        }
    }
}
