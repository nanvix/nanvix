// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    super::CLEANUP_SLEEP_DURATION,
    DEFAULT_APP_NAME,
    DEFAULT_TENANT_ID,
};
use crate::benchmark::{
    Benchmark,
    UserVmDeployment,
};
use ::anyhow::Result;
use ::indicatif::{
    ProgressBar,
    ProgressStyle,
};
use ::nanvix::http::message::New;
use ::reqwest::header::HeaderMap;
use ::std::time::Duration;

impl Benchmark {
    ///
    /// # Description
    ///
    /// This function runs the cold-start experiment, where we measure the time to start linuxd,
    /// start a VM, and send a request to the new VM.
    ///
    /// # Arguments
    ///
    /// - `uservm_deployment`: deployment mode for the user VM.
    ///
    pub async fn run_cold_start(&mut self, user_vm_deployment: &UserVmDeployment) -> Result<()> {
        // Start nanvixd once.
        self.setup();

        // Display a progress bar
        let pb: ProgressBar = ProgressBar::new(self.iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        let cleanup_sleep_duration: Duration = Duration::from_millis(CLEANUP_SLEEP_DURATION);

        let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        for iter in 0..self.iterations {
            // In this benchmark we measure the time to start both linuxd and the user VM, so we
            // give each iteration a different tenant id and application name.
            let tenant_id = format!("{DEFAULT_TENANT_ID}-{iter}");
            let app_name = format!("{DEFAULT_APP_NAME}-{iter}");

            // Get the right message for this new user VM.
            let (new_msg_headers, new_msg): (HeaderMap, New) =
                self.prepare_new_message(Some(tenant_id), Some(app_name))?;

            // In case we are pre-warming, we will run the user VM once without keeping track of
            // the time-elapsed.
            if *user_vm_deployment == UserVmDeployment::PreWarm {
                self.run_user_vm_echo_once(
                    new_msg_headers.clone(),
                    new_msg.clone(),
                    cleanup_sleep_duration,
                    None,
                    &mut None,
                )
                .await?;
            }

            self.run_user_vm_echo_once(
                new_msg_headers.clone(),
                new_msg.clone(),
                cleanup_sleep_duration,
                Some(&mut latencies),
                &mut None,
            )
            .await?;
            pb.inc(1);
        }

        pb.finish();
        println!("First req: {} us", latencies[0]);
        latencies.sort();
        println!("p50: {} us", latencies[(self.iterations as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(self.iterations as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(self.iterations as f32 * 0.99) as usize]);

        print!("Cleaning up...");
        self.cleanup();
        println!("done!");

        Ok(())
    }
}
