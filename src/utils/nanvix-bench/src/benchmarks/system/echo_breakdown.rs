// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::super::WARMUP_SLEEP_DURATION;
use crate::benchmark::Benchmark;
use ::anyhow::Result;
use ::indicatif::{
    ProgressBar,
    ProgressStyle,
};
use ::nanvix::syscomm::{
    ReadExact,
    WriteAll,
};
use ::std::time::Duration;
use ::tokio::time::sleep;

impl Benchmark {
    #[cfg(feature = "timestamp-messages")]
    pub async fn run_echo_breakdown(&mut self) -> Result<()> {
        // First start nanvixd and the user VM.
        let (new_msg_headers, new_msg) = self.prepare_new_message(None, None)?;
        self.setup();
        let (user_vm_id, mut gateway_stream) = self.start(new_msg, new_msg_headers).await?;

        // The labels in this array are also added as comments to the line of code where the
        // timestamp is added.
        let steps: Vec<&str> = vec![
            // In-path
            "nanvix-bench::write_all()",                    // 0
            "linuxd::worker_thread::handle_read_request()", // 1
            "uservm::io_thread::system_vm::read()",         // 2
            "uservm::memory_thread::data_rx::recv()",       // 3
            "uservm::lib::vm_input::vmexit()",              // 4
            "uservm::lib::vm_input::vm_write_bytes()",      // 5
            // Out-path
            "uservm::lib::vm_output::send()",                // 6
            "uservm::io_thread::system_vm::write()",         // 7
            "linuxd::worker_thread::handle_write_request()", // 8
            "nanvix-bench::read_exact()",                    // 9
        ];

        let header_size = 1;
        let data_size = header_size + profiler::MAX_NUMBER_MESSAGE_TIMESTAMPS * 2;

        // Warmup: send one untimed echo to trigger lazy initialization (worker thread
        // creation, TCP path warm-up, etc.) so that timed iterations reflect steady-state
        // latency.
        {
            let warmup_data: Vec<u8> = vec![0u8; data_size];
            let mut warmup_response: Vec<u8> = vec![0u8; data_size];
            gateway_stream.write_all(&warmup_data).await?;
            gateway_stream.read_exact(&mut warmup_response).await?;
            sleep(Duration::from_millis(WARMUP_SLEEP_DURATION)).await;
        }

        // For each different step we measure, we record the delta for each iteration.
        let mut latencies: Vec<Vec<u16>> = Vec::with_capacity(steps.len() + 1);
        for _ in 0..(steps.len() + 1) {
            latencies.push(vec![0u16; self.iterations]);
        }

        // Display a progress bar.
        let pb: ProgressBar = ProgressBar::new(self.iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        for iter in 0..self.iterations {
            let mut data: Vec<u8> = vec![0u8; data_size];
            let mut response: Vec<u8> = vec![0u8; data_size];

            // Add initial timestamp
            // Label: nanvix-bench::write_all()
            profiler::timestamp_message!(&mut data, 0);

            gateway_stream.write_all(&data).await?;
            gateway_stream.read_exact(&mut response).await?;

            // Add final timestamp.
            // Label: nanvix-bench::read_exact()
            profiler::timestamp_message!(&mut response, 0);

            // Process results.
            let mut first_timestamp: Option<u16> = None;
            let mut last_timestamp: Option<u16> = None;
            let num_stamps: usize = response[0] as usize;
            if num_stamps != steps.len() {
                return Err(anyhow::anyhow!(
                    "not enough timestamps (got={num_stamps}, expected={})",
                    steps.len()
                ));
            }
            for (step_idx, chunk) in (0..num_stamps).zip(response[header_size..].chunks_exact(2)) {
                let timestamp: u16 = u16::from_le_bytes([chunk[0], chunk[1]]);

                if first_timestamp.is_none() {
                    first_timestamp = Some(timestamp);
                }

                if let Some(last) = last_timestamp {
                    let delta: u16 = timestamp.wrapping_sub(last);
                    latencies[step_idx][iter] = delta;
                }

                last_timestamp = Some(timestamp);
            }

            if first_timestamp.is_some() && last_timestamp.is_some() {
                latencies[steps.len()][iter] = last_timestamp
                    .unwrap()
                    .wrapping_sub(first_timestamp.unwrap())
            } else {
                return Err(anyhow::anyhow!("have not collected enough timestamps!"));
            }

            pb.inc(1);
        }

        pb.finish();

        // Clean-up.
        self.kill(user_vm_id).await?;
        self.cleanup();

        // Print results
        for step_idx in 0..(steps.len() + 1) {
            if step_idx < steps.len() {
                print!("{step_idx:<2} | {:<48}", steps[step_idx]);
            } else {
                print!("{step_idx:<2} | {:<48}", "Total");
            }

            if step_idx == 0 {
                println!(" | First Step");
                continue;
            }

            latencies[step_idx].sort();
            print!(
                " | p50: {:5} | p95: {:5} | p99 {:5}",
                latencies[step_idx][(self.iterations as f32 * 0.5) as usize],
                latencies[step_idx][(self.iterations as f32 * 0.95) as usize],
                latencies[step_idx][(self.iterations as f32 * 0.99) as usize],
            );

            if step_idx < steps.len() && steps[step_idx] == "microvm::mod::vm_input::vmexit()" {
                println!(" | Time for VM to react to IO being avail.");
            } else {
                println!();
            }
        }

        Ok(())
    }
}
