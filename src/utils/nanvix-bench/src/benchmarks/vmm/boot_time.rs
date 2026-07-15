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
use ::std::time::Instant;
use ::tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::sleep,
};

impl Benchmark {
    /// This function runs the boot-time experiment, where we measure the time to start a user VM
    /// with a noop application and exit. To properly isolate just the time to start a user VM, we
    /// do not make use of nanvixd here. Instead, we start the user VM manually.
    pub async fn run_boot_time(&mut self) -> Result<()> {
        // Display a progress bar
        let pb = ProgressBar::new(self.iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

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

            let (io_handler_data_tx, memory_thread_data_rx) =
                mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);

            let kernel_filename: String =
                format!("{}/bin/kernel.elf", self.workspace_root.display());
            let initrd_filename: String = self.flavour.get_program(&self.workspace_root);

            // Create shared counters for tracking message flow across threads.
            let counters: MessageCounters = MessageCounters::new();

            let start = Instant::now();
            let user_vm_handle = UserVm::spawn(UserVmArgs {
                kernel_filename,
                initrd_filename: Some(initrd_filename),
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
                #[cfg(feature = "profile-time")]
                perf_timings: ::nanvix::uservm::perf::PerfTimings::new(),
                guest_profile_path: None,
            });

            let join_result = user_vm_handle.await;

            drop(io_handler_data_tx);
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
                            format!("error running user VM, exit-status={exit_status}");
                        error!("{reason}");
                        return Err(anyhow::anyhow!(reason));
                    }
                    debug!("User VM: done running");
                },
                Ok(Err(error)) => {
                    error!("error running user VM: {error:?}");
                    return Err(error);
                },
                Err(error) => {
                    let reason: String = format!("error joining user VM task: {error:?}");
                    error!("{reason}");
                    return Err(anyhow::anyhow!(reason));
                },
            }

            latencies.push(start.elapsed().as_micros());

            pb.inc(1);

            // Need to give some time to clean-up
            sleep(std::time::Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
        }

        pb.finish();
        latencies.sort();
        println!("p50: {} us", latencies[(self.iterations as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(self.iterations as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(self.iterations as f32 * 0.99) as usize]);

        Ok(())
    }
}
