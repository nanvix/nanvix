// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
    path::PathBuf,
    time::Instant,
};
use ::tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    process::Command,
    time::Duration,
};

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Payload sent to the echo program during each cold-start iteration.
///
const ECHO_PAYLOAD: &[u8] = b"hello\n";

///
/// # Description
///
/// Timeout (in seconds) for a single cold-start iteration. If the iteration exceeds this
/// duration, it is considered failed.
///
const COLD_START_TIMEOUT_SECS: u64 = 120;

//==================================================================================================
// Implementations
//==================================================================================================

impl Benchmark {
    ///
    /// # Description
    ///
    /// Runs the standalone cold-start benchmark. Each iteration spawns a fresh nanvixd process in
    /// interactive mode with the echo program, writes a payload to stdin, and measures the time
    /// until the echo response arrives on stdout.
    ///
    pub async fn run_cold_start_standalone(&mut self) -> Result<()> {
        let nanvixd_bin: PathBuf = self.standalone_nanvixd_path();
        let program: String = self.flavour.get_program(&self.workspace_root);

        if !nanvixd_bin.exists() {
            let reason: String = format!("nanvixd binary not found at {}", nanvixd_bin.display());
            error!("{reason}");
            anyhow::bail!(reason);
        }

        let pb: ProgressBar = ProgressBar::new(self.iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        for _ in 0..self.iterations {
            let latency: u128 = ::tokio::time::timeout(
                Duration::from_secs(COLD_START_TIMEOUT_SECS),
                self.run_standalone_iteration(&nanvixd_bin, &program),
            )
            .await
            .map_err(|_| {
                anyhow::anyhow!("cold-start iteration timed out after {COLD_START_TIMEOUT_SECS}s")
            })??;

            latencies.push(latency);
            pb.inc(1);
        }

        pb.finish();
        println!("First req: {} us", latencies[0]);
        latencies.sort();
        println!("p50: {} us", latencies[(self.iterations as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(self.iterations as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(self.iterations as f32 * 0.99) as usize]);

        Ok(())
    }

    /// Runs a single standalone cold-start iteration, returning the latency in microseconds.
    async fn run_standalone_iteration(&self, nanvixd_bin: &PathBuf, program: &str) -> Result<u128> {
        let start: Instant = Instant::now();

        let mut child: ::tokio::process::Child = Command::new(nanvixd_bin)
            .arg(::nanvixd::args::Args::OPT_SEPARATOR)
            .arg(program)
            .stdin(::std::process::Stdio::piped())
            .stdout(::std::process::Stdio::piped())
            .stderr(::std::process::Stdio::null())
            .current_dir(&self.workspace_root)
            .kill_on_drop(true)
            .spawn()?;

        // Write payload to stdin and drop it. Closing the pipe delivers EOF to the guest, causing
        // the echo program to exit after processing the payload.
        {
            let mut stdin: ::tokio::process::ChildStdin = child
                .stdin
                .take()
                .ok_or_else(|| anyhow::anyhow!("failed to take nanvixd stdin"))?;
            stdin.write_all(ECHO_PAYLOAD).await?;
        }

        // Read the echo response from stdout. The first data arriving confirms that the VM booted
        // and the echo program processed the payload.
        {
            let mut stdout: ::tokio::process::ChildStdout = child
                .stdout
                .take()
                .ok_or_else(|| anyhow::anyhow!("failed to take nanvixd stdout"))?;
            let mut buf: Vec<u8> = vec![0u8; ECHO_PAYLOAD.len()];
            stdout.read_exact(&mut buf).await?;
        }

        let elapsed: Duration = start.elapsed();

        // Wait for nanvixd to exit cleanly.
        let _ = child.wait().await;

        Ok(elapsed.as_micros())
    }

    /// Returns the platform-specific path to the nanvixd binary.
    fn standalone_nanvixd_path(&self) -> PathBuf {
        let bin_name: &str = if cfg!(windows) {
            "nanvixd.exe"
        } else {
            "nanvixd.elf"
        };
        self.workspace_root.join("bin").join(bin_name)
    }
}
