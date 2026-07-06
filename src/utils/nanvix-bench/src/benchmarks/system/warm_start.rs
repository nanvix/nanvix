// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::super::{
    CLEANUP_SLEEP_DURATION,
    WARMUP_SLEEP_DURATION,
};
use crate::benchmark::Benchmark;
use ::anyhow::Result;
use ::indicatif::{
    ProgressBar,
    ProgressStyle,
};
use ::log::error;
use ::nanvix::syscomm::{
    ReadExact,
    WriteAll,
};
use ::std::time::{
    Duration,
    Instant,
};
use ::tokio::time::sleep;

impl Benchmark {
    /// This function runs the warm start benchmark, where we measure the time to send a request
    /// into the VM once it has started executing.
    pub async fn run_warm_start(&mut self) -> Result<()> {
        // Display a progress bar
        let iterations: u64 = u64::try_from(self.iterations)
            .map_err(|e| anyhow::anyhow!("iteration count exceeds u64: {e}"))?;
        let pb: ProgressBar = ProgressBar::new(iterations);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .map_err(|e| anyhow::anyhow!("error creating progress bar: {e}"))?
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        // Payload we are sending over the wire
        let payload: Vec<u8> = vec![7u8; self.payload_size];

        let (new_msg_headers, new_msg) = self.prepare_new_message(None, None)?;

        // Start nanvixd.
        self.setup();

        // Start User VM.
        let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        let user_vm_id = {
            let (user_vm_id, mut gateway_stream) = self.start(new_msg, new_msg_headers).await?;
            let mut response_payload: Vec<u8> = vec![0u8; payload.len()];

            // Warmup: send one untimed echo to trigger lazy initialization (worker thread
            // creation, TCP path warm-up, etc.) so that timed iterations reflect steady-state
            // latency.
            {
                gateway_stream.write_all(&payload).await?;
                gateway_stream.read_exact(&mut response_payload).await?;
                sleep(Duration::from_millis(WARMUP_SLEEP_DURATION)).await;
            }

            for _ in 0..self.iterations {
                let start = Instant::now();
                gateway_stream.write_all(&payload).await?;
                gateway_stream.read_exact(&mut response_payload).await?;
                latencies.push(start.elapsed().as_micros());

                // Sanity-check the message to make sure is the same we sent.
                if response_payload != payload {
                    error!("received payload does not match sent payload!");
                    error!(" - sent: {payload:?}");
                    error!(" - got: {response_payload:?}");
                }

                pb.inc(1);
                sleep(Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
            }
            user_vm_id
        };

        // Kill the user VM.
        self.kill(user_vm_id).await?;

        // Stop nanvixd.
        self.cleanup();

        pb.finish();
        println!("First req: {} us", latencies[0]);
        latencies.sort();
        println!("p50: {} us", latencies[(self.iterations as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(self.iterations as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(self.iterations as f32 * 0.99) as usize]);

        Ok(())
    }
}
