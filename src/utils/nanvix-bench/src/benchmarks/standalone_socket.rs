// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Standalone deployment of the `warm-start-socket` benchmark.
//!
//! Spawns a `nanvixd` process in standalone mode with host networking enabled (so its in-process
//! `networkd` opens real host-namespace sockets), then drives the shared TCP echo client to measure
//! round-trip latency over the full networking path. The `nanvixd` process is killed once the
//! measurement completes; this mirrors the standalone `cold-start` benchmark's process lifecycle
//! and, unlike an in-process VM handle, tears down cleanly even though the guest echo server loops
//! forever.

//==================================================================================================
// Imports
//==================================================================================================

use super::socket_echo::GUEST_ECHO_PORT;
use crate::benchmark::Benchmark;
use ::anyhow::Result;
use ::std::{
    net::TcpListener,
    path::PathBuf,
    process::Stdio,
};
use ::tokio::{
    process::{
        Child,
        Command,
    },
    time::{
        Duration,
        timeout,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum time to wait for `nanvixd` to exit after asking Tokio to kill it.
const NANVIXD_TEARDOWN_TIMEOUT_SECS: u64 = 5;

//==================================================================================================
// Implementations
//==================================================================================================

impl Benchmark {
    /// Runs the `warm-start-socket` benchmark in standalone mode.
    pub(crate) async fn run_warm_start_socket_standalone(&mut self) -> Result<()> {
        let nanvixd_bin = self.standalone_nanvixd_path();
        if !nanvixd_bin.exists() {
            anyhow::bail!(
                "nanvixd binary not found at {} (build the nanvixd target first)",
                nanvixd_bin.display()
            );
        }

        let program: String = self.flavour.get_program(&self.workspace_root);
        let program_path: PathBuf = PathBuf::from(&program);
        if !program_path.exists() {
            anyhow::bail!("benchmark program not found at {}", program_path.display());
        }
        Self::ensure_guest_echo_port_available()?;

        // Boot the guest via nanvixd with host networking enabled. The guest echo server ignores
        // stdin/stdout and communicates exclusively over its network socket; stderr is inherited
        // so startup failures remain visible.
        // `kill_on_drop` guarantees the VM is torn down even on an early return.
        let mut cmd: Command = Command::new(&nanvixd_bin);
        cmd.arg(::nanvixd::args::Args::OPT_ALLOW_HOST_NETWORKING)
            .arg(::nanvixd::args::Args::OPT_SEPARATOR)
            .arg(&program)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .current_dir(&self.workspace_root)
            .kill_on_drop(true);

        let mut child: Child = cmd.spawn()?;

        let result: Result<()> = self.run_socket_echo_client().await;

        // Tear down the VM regardless of the client outcome.
        let teardown_result: Result<()> = Self::teardown_socket_nanvixd(&mut child).await;

        match (result, teardown_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) => Err(error),
            (Ok(()), Err(error)) => Err(error),
            (Err(client_error), Err(teardown_error)) => Err(anyhow::anyhow!(
                "{client_error}; additionally failed to tear down nanvixd: {teardown_error}"
            )),
        }
    }

    async fn teardown_socket_nanvixd(child: &mut Child) -> Result<()> {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(());
            }
            anyhow::bail!("nanvixd exited before teardown with status: {status}");
        }

        child.start_kill()?;
        match timeout(Duration::from_secs(NANVIXD_TEARDOWN_TIMEOUT_SECS), child.wait()).await {
            Ok(status) => {
                status?;
                Ok(())
            },
            Err(_) => anyhow::bail!(
                "timed out waiting {NANVIXD_TEARDOWN_TIMEOUT_SECS}s for nanvixd to exit after kill"
            ),
        }
    }

    fn ensure_guest_echo_port_available() -> Result<()> {
        let listener: TcpListener =
            TcpListener::bind(("127.0.0.1", GUEST_ECHO_PORT)).map_err(|error| {
                anyhow::anyhow!(
                    "guest echo port 127.0.0.1:{GUEST_ECHO_PORT} is unavailable; another \
                     warm-start-socket benchmark or stale listener may be running: {error}"
                )
            })?;
        drop(listener);

        Ok(())
    }
}
