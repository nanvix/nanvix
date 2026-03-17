// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
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
use ::std::time::Instant;
use ::tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::sleep,
};

impl Benchmark {
    ///
    /// # Description
    ///
    /// Runs the snapshot-restore experiment. The setup phase creates a snapshot using the
    /// `snapshot-rust-nostd.elf` program, and the measurement phase iteratively restores from
    /// that snapshot, measuring the time for each restore-and-exit cycle.
    ///
    pub async fn run_snapshot_restore(&mut self) -> Result<()> {
        let kernel_filename: String = format!("{}/bin/kernel.elf", self.workspace_root.display());
        let snapshot_program: String = self.flavour.get_program(&self.workspace_root);

        // Ensure snapshots directory exists.
        let snapshots_dir: std::path::PathBuf = self.workspace_root.join("snapshots");
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
                ramfs_filename: None,
                stderr: Some("/dev/null".to_string()),
                vcpu_thread_stdout_tx,
                memory_thread_data_rx,
                io_control_rx,
                io_control_tx,
                counters,
                snapshot_path: None,
                #[cfg(feature = "gdb")]
                gdb_port: None,
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

            let start = Instant::now();
            let user_vm_handle = UserVm::spawn(UserVmArgs {
                kernel_filename: kernel_filename.clone(),
                initrd_filename: None,
                initrd_args: None,
                ramfs_filename: None,
                stderr: Some("/dev/null".to_string()),
                vcpu_thread_stdout_tx,
                memory_thread_data_rx,
                io_control_rx,
                io_control_tx,
                counters,
                snapshot_path: Some(kernel_filename.clone()),
                #[cfg(feature = "gdb")]
                gdb_port: None,
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

            pb.inc(1);

            sleep(std::time::Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
        }

        pb.finish();
        latencies.sort();
        println!("p50: {} us", latencies[(self.iterations * 50) / 100]);
        println!("p95: {} us", latencies[(self.iterations * 95) / 100]);
        println!("p99: {} us", latencies[(self.iterations * 99) / 100]);

        Ok(())
    }
}
