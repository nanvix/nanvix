// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::super::CLEANUP_SLEEP_DURATION;
use crate::benchmark::Benchmark;
use ::anyhow::Result;
use ::indicatif::{
    ProgressBar,
    ProgressStyle,
};
use ::log::{
    error,
    warn,
};
use ::nanvix::{
    http::message::New,
    syscomm::{
        ReadExact,
        WriteAll,
    },
};
use ::reqwest::header::HeaderMap;
use ::std::{
    collections::HashMap,
    time::{
        Duration,
        Instant,
    },
};
use ::tokio::time::sleep;

impl Benchmark {
    ///
    /// # Description
    ///
    /// This function runs the round-trip latency benchmark, where we measure the latency of
    /// sending one message and getting it back, as we increase the message size.
    ///
    /// Results are reported as p50, p95, and p99 percentiles for each message size.
    ///
    pub async fn run_round_trip_latency(&mut self) -> Result<()> {
        let message_sizes: Vec<(&str, u64)> = vec![
            ("32 B", 32),
            ("64 B", 64),
            ("128 B", 128),
            ("256 B", 256),
            ("512 B", 512),
            ("1 KiB", 1024),
            ("4 KiB", 4 * 1024),
        ];

        // Display a progress bar
        let total_num_iters: usize = self.iterations * message_sizes.len();
        let pb: ProgressBar = ProgressBar::new(total_num_iters as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        let (new_msg_headers, new_msg): (HeaderMap, New) = self.prepare_new_message(None, None)?;

        // Start nanvixd.
        self.setup();

        let mut latencies: HashMap<&str, Vec<u128>> = HashMap::new();
        let user_vm_id = {
            // Start User VM.
            let (user_vm_id, mut gateway_stream) = self.start(new_msg, new_msg_headers).await?;

            // Iterate over all possible message sizes.
            for (label, message_size) in &message_sizes {
                let payload: Vec<u8> = vec![7u8; *message_size as usize];

                // For each message size send many messages to get statistically relevant results.
                for _ in 0..self.iterations {
                    let mut response_payload: Vec<u8> = vec![0u8; *message_size as usize];

                    let start: Instant = Instant::now();
                    gateway_stream.write_all(&payload).await?;
                    gateway_stream.read_exact(&mut response_payload).await?;
                    latencies
                        .entry(label)
                        .or_default()
                        .push(start.elapsed().as_micros());

                    // Sanity-check the message to make sure is the same we sent.
                    if response_payload != payload {
                        error!("received payload does not match sent payload!");
                        error!(" - sent: {payload:?}");
                        error!(" - got: {response_payload:?}");
                    }

                    pb.inc(1);
                    sleep(Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
                }
            }
            user_vm_id
        };

        // Kill the user VM.
        self.kill(user_vm_id).await?;

        // Stop nanvixd.
        self.cleanup();

        pb.finish();
        println!("Size:\tp50\tp95\tp99 [us]");
        // Iterate over the message size list to print the labels in order.
        for (label, _) in message_sizes.iter() {
            if let Some(latencies) = latencies.get_mut(label) {
                latencies.sort();
                let p50: u128 = latencies[(self.iterations as f32 * 0.5) as usize];
                let p95: u128 = latencies[(self.iterations as f32 * 0.95) as usize];
                let p99: u128 = latencies[(self.iterations as f32 * 0.99) as usize];
                println!("{label}:\t{p50}\t{p95}\t{p99}");
            } else {
                warn!("missing latencies for message size: {label}");
            }
        }

        Ok(())
    }
}
