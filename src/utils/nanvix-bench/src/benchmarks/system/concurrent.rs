// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::super::CLEANUP_SLEEP_DURATION;
use crate::benchmark::Benchmark;
use ::anyhow::Result;
use ::indicatif::{
    ProgressBar,
    ProgressStyle,
};
use ::log::error;
use ::nanvix::{
    http::message::New,
    sandbox::UserVmIdentifier,
    syscomm::SocketStream,
};
use ::reqwest::header::HeaderMap;
use ::std::time::Duration;

impl Benchmark {
    ///
    /// # Description
    ///
    /// This benchmark measures the time to start N concurrent user VMs, all sharing the same
    /// linuxd instance.
    ///
    /// # Arguments
    ///
    /// - `num_concurrent_vms`: number of VMs to start concurrently.
    ///
    pub async fn run_concurrent(&mut self, num_concurrent_vms: usize) -> Result<()> {
        // Display a progress bar
        let pb: ProgressBar = ProgressBar::new(num_concurrent_vms.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        // Start nanvixd once.
        self.setup();

        let cleanup_sleep_duration: Duration = Duration::from_millis(CLEANUP_SLEEP_DURATION);

        let mut latencies: Vec<u128> = Vec::with_capacity(num_concurrent_vms);
        let mut in_flight_uvms: Option<Vec<(UserVmIdentifier, SocketStream)>> =
            Some(Vec::with_capacity(num_concurrent_vms));
        for iter in 0..num_concurrent_vms {
            // In this benchmark we want all user VMs to share the same linuxd instance, so they
            // run concurrently. We therefore keep the tenant id constant (default) and give
            // each user VM a different name.
            let app_name: String = format!("bar-{iter}");
            let (new_msg_headers, new_msg): (HeaderMap, New) =
                self.prepare_new_message(None, Some(app_name))?;

            // We want all user VMs to run concurrently, so we pass an in-flight map to keep them
            // around instead of killing them after getting the echo.
            self.run_user_vm_echo_once(
                new_msg_headers,
                new_msg,
                cleanup_sleep_duration,
                Some(&mut latencies),
                &mut in_flight_uvms,
            )
            .await?;
            pb.inc(1);
        }

        pb.finish();
        println!(
            "Time to spawn {num_concurrent_vms} user VMs (in serial): {:.2} s",
            (latencies.iter().sum::<u128>() as f64) / 1_000_000.0
        );
        latencies.sort();
        println!("p50: {} us", latencies[(num_concurrent_vms as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(num_concurrent_vms as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(num_concurrent_vms as f32 * 0.99) as usize]);

        print!("Cleaning up...");

        if let Some(in_flight_uvms) = in_flight_uvms.as_mut() {
            for (user_vm_id, _) in in_flight_uvms.drain(..) {
                self.kill(user_vm_id).await?;
            }
        } else {
            error!("in_flight_uvms cannot be none");
        }
        self.cleanup();

        println!("done!");

        Ok(())
    }
}
