// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::super::{
    CHANNEL_CAPACITY,
    CLEANUP_SLEEP_DURATION,
};
use crate::benchmark::Benchmark;
use ::anyhow::Result;
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
#[cfg(feature = "profile-time")]
use ::std::collections::HashMap;
use ::std::time::Instant;
use ::tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::sleep,
};

/// Phase names in display order for the timing breakdown table.
#[cfg(feature = "profile-time")]
const PHASE_NAMES: &[&str] = &[
    "partition_create_us",
    "vmem_create_us",
    "vcpu_create_us",
    "kernel_load_us",
    "vcpu_reset_us",
    "thread_spawn_us",
    "guest_exec_us",
    "exit_handling_us",
    "total_us",
];

/// Human-readable labels for each phase, matching [`PHASE_NAMES`] order.
#[cfg(feature = "profile-time")]
const PHASE_LABELS: &[&str] = &[
    "partition_create",
    "vmem_create",
    "vcpu_create",
    "kernel_load",
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

        // Phase 1: Create a snapshot by running the snapshot program.
        println!("Creating snapshot...");
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

            let (_io_thread_data_tx, memory_thread_data_rx) =
                mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);

            let counters: MessageCounters = MessageCounters::new();

            let user_vm_handle = UserVm::spawn(UserVmArgs {
                kernel_filename: kernel_filename.clone(),
                initrd_filename: Some(snapshot_program),
                initrd_args: None,
                kernel_args: Some(::koptions::SNAPSHOT_TOKEN.to_string()),
                ramfs_filename: None,
                stderr: Some(if cfg!(windows) { "NUL" } else { "/dev/null" }.to_string()),
                vcpu_thread_stdout_tx,
                memory_thread_data_rx,
                io_control_rx,
                io_control_tx,
                counters,
                snapshot_path: None,
                mount_directory: None,
                #[cfg(feature = "gdb")]
                gdb_port: None,
                #[cfg(feature = "profile-time")]
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
                            format!("error creating snapshot, exit-status={exit_status}");
                        error!("{reason}");
                        return Err(anyhow::anyhow!(reason));
                    }
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

        // Phase 2: Measure snapshot restore latency.
        let pb = ProgressBar::new(self.iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Snapshot restore:");

        let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        #[cfg(feature = "profile-time")]
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

            let (_io_thread_data_tx, memory_thread_data_rx) =
                mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);

            let counters: MessageCounters = MessageCounters::new();

            #[cfg(feature = "profile-time")]
            let perf_timings = ::nanvix::uservm::perf::PerfTimings::new();
            #[cfg(feature = "profile-time")]
            let perf_reader = perf_timings.clone();

            let start = Instant::now();
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
                mount_directory: None,
                #[cfg(feature = "gdb")]
                gdb_port: None,
                #[cfg(feature = "profile-time")]
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

            latencies.push(start.elapsed().as_micros());

            // Accumulate per-phase timing samples.
            #[cfg(feature = "profile-time")]
            {
                let json_str: String = perf_reader.to_json();
                if let Ok(timings) = ::serde_json::from_str::<::serde_json::Value>(&json_str) {
                    for name in PHASE_NAMES {
                        if let Some(value) = timings.get(*name) {
                            if let Some(samples) = phase_samples.get_mut(*name) {
                                if let Some(v) = value.as_u64() {
                                    samples.push(v);
                                }
                            }
                        }
                    }
                }
            }

            pb.inc(1);

            sleep(std::time::Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
        }

        pb.finish();
        println!("First req: {} us", latencies[0]);
        latencies.sort();
        let len: usize = latencies.len();
        let p50: u128 = latencies[((len as f32 * 0.5) as usize).min(len - 1)];
        let p95: u128 = latencies[((len as f32 * 0.95) as usize).min(len - 1)];
        let p99: u128 = latencies[((len as f32 * 0.99) as usize).min(len - 1)];
        let min: u128 = latencies[0];
        let max: u128 = latencies[len - 1];
        let mean: u128 = latencies.iter().sum::<u128>() / len as u128;
        println!("p50: {} us", p50);
        println!("p95: {} us", p95);
        println!("p99: {} us", p99);
        println!("min: {} us", min);
        println!("max: {} us", max);
        println!("mean: {} us", mean);

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
}
