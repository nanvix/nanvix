// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::super::{
    CHANNEL_CAPACITY,
    CLEANUP_SLEEP_DURATION,
};
use crate::benchmark::Benchmark;
use ::anyhow::{
    Context,
    Result,
};
use ::indicatif::{
    ProgressBar,
    ProgressStyle,
};
use ::log::{
    debug,
    error,
};
use ::nanvix::{
    sys::ipc::IkcFrame,
    uservm::{
        UserVm,
        UserVmArgs,
        counters::MessageCounters,
        orchestrator::{
            IoControlCommand,
            IoControlResponse,
        },
    },
};
use ::std::{
    collections::HashMap,
    time::Instant,
};
use ::tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::sleep,
};

/// Phase names in display order for the timing breakdown table.
const PHASE_NAMES: &[&str] = &[
    "partition_create_us",
    "vmem_create_us",
    "vcpu_create_us",
    "kernel_load_us",
    "snapshot_restore_us",
    "vcpu_reset_us",
    "thread_spawn_us",
    "guest_exec_us",
    "exit_handling_us",
    "total_us",
];

/// Human-readable labels for each phase, matching [`PHASE_NAMES`] order.
const PHASE_LABELS: &[&str] = &[
    "partition_create",
    "vmem_create",
    "vcpu_create",
    "kernel_load",
    "snapshot_restore",
    "vcpu_reset",
    "thread_spawn",
    "guest_exec",
    "exit_handling",
    "total",
];

impl Benchmark {
    ///
    /// # Description
    ///
    /// Runs the snapshot-restore experiment. The setup phase creates a snapshot using the
    /// `snapshot-rust-nostd.elf` program, and the measurement phase iteratively restores from
    /// that snapshot, measuring the time for each restore-and-exit cycle.
    ///
    /// Requires the `profile-time` feature to be enabled; all reported per-phase numbers are
    /// taken from the VMM's `PerfTimings` counters.
    ///
    pub async fn run_snapshot_restore(&mut self) -> Result<()> {
        if self.iterations == 0 {
            anyhow::bail!("run_snapshot_restore(): iterations must be at least 1");
        }

        let kernel_filename: String = format!("{}/bin/kernel.elf", self.workspace_root.display());
        let snapshot_program: String = self.flavour.get_program(&self.workspace_root);

        // Ensure snapshots directory exists.
        // NOTE: Uses a relative path to stay consistent with the VMM snapshot logic
        // (`whp::WhpHandle::make_snapshot_paths`), which writes to `snapshots/` relative to CWD.
        let snapshots_dir: std::path::PathBuf = std::path::PathBuf::from("snapshots");
        if !snapshots_dir.exists() {
            std::fs::create_dir_all(&snapshots_dir)?;
        }

        // Phase 1: Measure cold-start latency (no snapshot, just boot + workload + exit).
        let pb_cold = ProgressBar::new(self.iterations.try_into().unwrap());
        pb_cold.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb_cold.set_message("Cold-start:");

        let mut cold_start_latencies: Vec<u128> = Vec::with_capacity(self.iterations);

        for _ in 0..self.iterations {
            let (vcpu_thread_stdout_tx, mut vcpu_thread_stdout_rx) =
                mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
            let stdout_drain: JoinHandle<()> =
                ::tokio::spawn(
                    async move { while vcpu_thread_stdout_rx.recv().await.is_some() {} },
                );

            let (io_control_command_tx, io_control_rx) =
                mpsc::channel::<IoControlCommand>(CHANNEL_CAPACITY);
            let (io_control_tx, mut io_control_response_rx) =
                mpsc::channel::<IoControlResponse>(CHANNEL_CAPACITY);
            let io_response_drain: JoinHandle<()> =
                ::tokio::spawn(
                    async move { while io_control_response_rx.recv().await.is_some() {} },
                );

            let (_io_handler_data_tx, memory_thread_data_rx) =
                mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);

            let counters: MessageCounters = MessageCounters::new();

            let cold_start = Instant::now();
            let user_vm_handle = UserVm::spawn(UserVmArgs {
                kernel_filename: kernel_filename.clone(),
                initrd_filename: Some(snapshot_program.clone()),
                initrd_args: None,
                kernel_args: None,
                ramfs_filename: None,
                stderr: Some(if cfg!(windows) { "NUL" } else { "/dev/null" }.to_string()),
                vcpu_thread_stdout_tx,
                memory_thread_data_rx,
                io_control_rx,
                io_control_tx,
                counters,
                snapshot_path: None,
                #[cfg(feature = "gdb")]
                gdb_port: None,
                perf_timings: ::nanvix::uservm::perf::PerfTimings::new(),
                guest_profile_path: None,
            });

            let join_result = user_vm_handle.await;

            drop(io_control_command_tx);

            if let Err(error) = stdout_drain.await {
                error!("error draining user VM stdout channel: {error:?}");
            }
            if let Err(error) = io_response_drain.await {
                error!("error draining user VM control channel: {error:?}");
            }

            match join_result {
                Ok(Ok(exit_status)) => {
                    if exit_status != 0 {
                        let reason: String =
                            format!("error in cold-start run, exit-status={exit_status}");
                        error!("{reason}");
                        return Err(anyhow::anyhow!(reason));
                    }
                    cold_start_latencies.push(cold_start.elapsed().as_micros());
                    debug!("Cold-start measurement completed");
                },
                Ok(Err(error)) => {
                    error!("error in cold-start run: {error:?}");
                    return Err(error);
                },
                Err(error) => {
                    let reason: String = format!("error joining cold-start VM task: {error:?}");
                    error!("{reason}");
                    return Err(anyhow::anyhow!(reason));
                },
            }

            pb_cold.inc(1);

            sleep(std::time::Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
        }

        pb_cold.finish();

        // Phase 2: Create a snapshot by running the snapshot program with --snapshot.
        println!("Creating snapshot...");
        let snapshot_creation_us: u128;
        let snapshot_run_wallclock_us: u128;
        {
            let (vcpu_thread_stdout_tx, mut vcpu_thread_stdout_rx) =
                mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
            let stdout_drain: JoinHandle<()> =
                ::tokio::spawn(
                    async move { while vcpu_thread_stdout_rx.recv().await.is_some() {} },
                );

            let (io_control_command_tx, io_control_rx) =
                mpsc::channel::<IoControlCommand>(CHANNEL_CAPACITY);
            let (io_control_tx, mut io_control_response_rx) =
                mpsc::channel::<IoControlResponse>(CHANNEL_CAPACITY);
            let io_response_drain: JoinHandle<()> =
                ::tokio::spawn(
                    async move { while io_control_response_rx.recv().await.is_some() {} },
                );

            let (_io_handler_data_tx, memory_thread_data_rx) =
                mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);

            let counters: MessageCounters = MessageCounters::new();

            let creation_perf_timings = ::nanvix::uservm::perf::PerfTimings::new();
            let creation_perf_reader = creation_perf_timings.clone();

            let snapshot_start = Instant::now();
            let user_vm_handle = UserVm::spawn(UserVmArgs {
                kernel_filename: kernel_filename.clone(),
                initrd_filename: Some(snapshot_program),
                initrd_args: Some("--snapshot".to_string()),
                kernel_args: Some(::koptions::SNAPSHOT_TOKEN.to_string()),
                ramfs_filename: None,
                stderr: Some(if cfg!(windows) { "NUL" } else { "/dev/null" }.to_string()),
                vcpu_thread_stdout_tx,
                memory_thread_data_rx,
                io_control_rx,
                io_control_tx,
                counters,
                snapshot_path: None,
                #[cfg(feature = "gdb")]
                gdb_port: None,
                perf_timings: creation_perf_timings,
                guest_profile_path: None,
            });

            let join_result = user_vm_handle.await;

            drop(io_control_command_tx);

            if let Err(error) = stdout_drain.await {
                error!("error draining user VM stdout channel: {error:?}");
            }
            if let Err(error) = io_response_drain.await {
                error!("error draining user VM control channel: {error:?}");
            }

            match join_result {
                Ok(Ok(exit_status)) => {
                    if exit_status != 0 {
                        let reason: String =
                            format!("error creating snapshot, exit-status={exit_status}");
                        error!("{reason}");
                        return Err(anyhow::anyhow!(reason));
                    }
                    snapshot_run_wallclock_us = snapshot_start.elapsed().as_micros();
                    // The per-phase `snapshot_creation_us` recorded by the VMM brackets only the
                    // save operation, isolating the snapshot work from boot, the pre/post-snapshot
                    // guest workload, and process teardown.
                    let timings: ::serde_json::Value =
                        ::serde_json::from_str(&creation_perf_reader.to_json())
                            .context("failed to parse perf-timings JSON for snapshot creation")?;
                    snapshot_creation_us = timings
                        .get("snapshot_creation_us")
                        .and_then(|v| v.as_u64())
                        .map(u128::from)
                        .context("missing `snapshot_creation_us` in perf-timings")?;
                    debug!("Snapshot created successfully");
                },
                Ok(Err(error)) => {
                    error!("error creating snapshot: {error:?}");
                    return Err(error);
                },
                Err(error) => {
                    let reason: String = format!("error joining snapshot VM task: {error:?}");
                    error!("{reason}");
                    return Err(anyhow::anyhow!(reason));
                },
            }

            sleep(std::time::Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
        }

        // Report snapshot file sizes.
        // NOTE: Uses the same relative path as the VMM snapshot logic above.
        {
            let snapshots_dir: std::path::PathBuf = std::path::PathBuf::from("snapshots");
            if snapshots_dir.exists() {
                println!("Snapshot files:");
                for entry in std::fs::read_dir(&snapshots_dir)? {
                    let entry = entry?;
                    let size: u64 = entry.metadata()?.len();
                    println!(
                        "  {}: {} KB ({} bytes)",
                        entry.file_name().to_string_lossy(),
                        size / 1024,
                        size
                    );
                }
            }
        }

        println!("Cold-start (boot + workload + exit):");
        {
            cold_start_latencies.sort();
            let len: usize = cold_start_latencies.len();
            let p50: u128 = cold_start_latencies[((len as f32 * 0.5) as usize).min(len - 1)];
            let p95: u128 = cold_start_latencies[((len as f32 * 0.95) as usize).min(len - 1)];
            let p99: u128 = cold_start_latencies[((len as f32 * 0.99) as usize).min(len - 1)];
            let min: u128 = cold_start_latencies[0];
            let max: u128 = cold_start_latencies[len - 1];
            let mean: u128 = cold_start_latencies.iter().sum::<u128>() / len as u128;
            println!("  p50: {} us", p50);
            println!("  p95: {} us", p95);
            println!("  p99: {} us", p99);
            println!("  min: {} us", min);
            println!("  max: {} us", max);
            println!("  mean: {} us", mean);
        }
        println!(
            "Snapshot creation (VMM save: memory + CPU state serialization): {} us",
            snapshot_creation_us
        );
        println!(
            "Snapshot-creation run wall-clock (boot + workload + snapshot + post-walk + exit): {} \
             us",
            snapshot_run_wallclock_us
        );

        // Phase 3: Measure snapshot restore latency.
        let pb = ProgressBar::new(self.iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Snapshot restore:");

        let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        // Post-restore execution latency: time from the snapshot-restore handoff to vCPU until
        // the guest exits (i.e., `guest_exec_us + exit_handling_us`). This isolates the cost of
        // the workload that runs *after* the VMM has finished restoring state, separately from
        // the snapshot-restore step itself.
        let mut post_restore_latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        let mut phase_samples: HashMap<String, Vec<u64>> = {
            let mut m: HashMap<String, Vec<u64>> = HashMap::new();
            for name in PHASE_NAMES {
                m.insert((*name).to_string(), Vec::with_capacity(self.iterations));
            }
            m
        };

        for _ in 0..self.iterations {
            let (vcpu_thread_stdout_tx, mut vcpu_thread_stdout_rx) =
                mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
            let stdout_drain: JoinHandle<()> =
                ::tokio::spawn(
                    async move { while vcpu_thread_stdout_rx.recv().await.is_some() {} },
                );

            let (io_control_command_tx, io_control_rx) =
                mpsc::channel::<IoControlCommand>(CHANNEL_CAPACITY);
            let (io_control_tx, mut io_control_response_rx) =
                mpsc::channel::<IoControlResponse>(CHANNEL_CAPACITY);
            let io_response_drain: JoinHandle<()> =
                ::tokio::spawn(
                    async move { while io_control_response_rx.recv().await.is_some() {} },
                );

            let (_io_handler_data_tx, memory_thread_data_rx) =
                mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);

            let counters: MessageCounters = MessageCounters::new();

            let perf_timings = ::nanvix::uservm::perf::PerfTimings::new();
            let perf_reader = perf_timings.clone();

            let user_vm_handle = UserVm::spawn(UserVmArgs {
                kernel_filename: kernel_filename.clone(),
                initrd_filename: None,
                initrd_args: None,
                kernel_args: None,
                ramfs_filename: None,
                stderr: Some(if cfg!(windows) { "NUL" } else { "/dev/null" }.to_string()),
                vcpu_thread_stdout_tx,
                memory_thread_data_rx,
                io_control_rx,
                io_control_tx,
                counters,
                snapshot_path: Some(kernel_filename.clone()),
                #[cfg(feature = "gdb")]
                gdb_port: None,
                perf_timings,
                guest_profile_path: None,
            });

            let join_result = user_vm_handle.await;

            drop(io_control_command_tx);

            if let Err(error) = stdout_drain.await {
                error!("error draining user VM stdout channel: {error:?}");
            }
            if let Err(error) = io_response_drain.await {
                error!("error draining user VM control channel: {error:?}");
            }

            match join_result {
                Ok(Ok(exit_status)) => {
                    if exit_status != 0 {
                        let reason: String =
                            format!("error restoring snapshot, exit-status={exit_status}");
                        error!("{reason}");
                        return Err(anyhow::anyhow!(reason));
                    }
                    debug!("Snapshot restore: done running");
                },
                Ok(Err(error)) => {
                    error!("error restoring snapshot: {error:?}");
                    return Err(error);
                },
                Err(error) => {
                    let reason: String = format!("error joining snapshot restore task: {error:?}");
                    error!("{reason}");
                    return Err(anyhow::anyhow!(reason));
                },
            }

            // Parse the per-phase breakdown and report
            // `snapshot_restore + guest_exec + exit_handling` as the headline latency.
            let timings: ::serde_json::Value = ::serde_json::from_str(&perf_reader.to_json())
                .context("failed to parse perf-timings JSON for snapshot restore")?;

            let snapshot_restore: u64 = timings
                .get("snapshot_restore_us")
                .and_then(|v| v.as_u64())
                .context("missing `snapshot_restore_us` in perf-timings")?;
            let guest_exec: u64 = timings
                .get("guest_exec_us")
                .and_then(|v| v.as_u64())
                .context("missing `guest_exec_us` in perf-timings")?;
            let exit_handling: u64 = timings
                .get("exit_handling_us")
                .and_then(|v| v.as_u64())
                .context("missing `exit_handling_us` in perf-timings")?;

            latencies.push(
                u128::from(snapshot_restore) + u128::from(guest_exec) + u128::from(exit_handling),
            );
            post_restore_latencies.push(u128::from(guest_exec) + u128::from(exit_handling));

            // Accumulate per-phase timing samples.
            for name in PHASE_NAMES {
                if let Some(value) = timings.get(*name)
                    && let Some(samples) = phase_samples.get_mut(*name)
                    && let Some(v) = value.as_u64()
                {
                    samples.push(v);
                }
            }

            pb.inc(1);

            sleep(std::time::Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
        }

        pb.finish();
        println!("Snapshot restore (snapshot restore + guest execution + exit handling):");
        println!("  First: {} us", latencies[0]);
        latencies.sort();
        let len: usize = latencies.len();
        let p50: u128 = latencies[((len as f32 * 0.5) as usize).min(len - 1)];
        let p95: u128 = latencies[((len as f32 * 0.95) as usize).min(len - 1)];
        let p99: u128 = latencies[((len as f32 * 0.99) as usize).min(len - 1)];
        let min: u128 = latencies[0];
        let max: u128 = latencies[len - 1];
        let mean: u128 = latencies.iter().sum::<u128>() / len as u128;
        println!("  p50: {} us", p50);
        println!("  p95: {} us", p95);
        println!("  p99: {} us", p99);
        println!("  min: {} us", min);
        println!("  max: {} us", max);
        println!("  mean: {} us", mean);

        // Post-restore execution: isolates the workload that runs after the VMM finishes
        // restoring state (`guest_exec_us + exit_handling_us`).
        println!("Post-restore execution (guest execution + exit handling):");
        println!("  First: {} us", post_restore_latencies[0]);
        post_restore_latencies.sort();
        let pr_len: usize = post_restore_latencies.len();
        let pr_p50: u128 = post_restore_latencies[((pr_len as f32 * 0.5) as usize).min(pr_len - 1)];
        let pr_p95: u128 =
            post_restore_latencies[((pr_len as f32 * 0.95) as usize).min(pr_len - 1)];
        let pr_p99: u128 =
            post_restore_latencies[((pr_len as f32 * 0.99) as usize).min(pr_len - 1)];
        let pr_min: u128 = post_restore_latencies[0];
        let pr_max: u128 = post_restore_latencies[pr_len - 1];
        let pr_mean: u128 = post_restore_latencies.iter().sum::<u128>() / pr_len as u128;
        println!("  p50: {} us", pr_p50);
        println!("  p95: {} us", pr_p95);
        println!("  p99: {} us", pr_p99);
        println!("  min: {} us", pr_min);
        println!("  max: {} us", pr_max);
        println!("  mean: {} us", pr_mean);

        // Per-phase timing breakdown.
        println!();
        println!("{:<22} {:>10} {:>10} {:>10}", "Phase", "p50 (us)", "p95 (us)", "p99 (us)");
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

        Ok(())
    }
}
