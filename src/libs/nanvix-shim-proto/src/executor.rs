// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(clippy::needless_return)]

//==================================================================================================
// Imports
//==================================================================================================

use std::{
    io::Write,
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
};

use crate::{
    args::ShimArgs,
    sandbox_service::NanvixSandboxService,
    sys,
    task_service::NanvixTaskService,
};
use containerd_shim_protos::{
    sandbox_async,
    shim_async,
    ttrpc,
};
use nanvix_shim_core::runtime::WorkloadRuntime;

//==================================================================================================
// Types
//==================================================================================================

/// The shim executor handles the shimv2 binary protocol commands.
pub struct ShimExecutor {
    /// Command-line arguments passed to the shim.
    pub args: ShimArgs,
    /// Standalone workload execution implementation.
    pub runtime: Arc<dyn WorkloadRuntime>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ShimExecutor {
    /// Create a new shim executor.
    pub fn new(args: ShimArgs, runtime: Arc<dyn WorkloadRuntime>) -> Self {
        Self { args, runtime }
    }

    /// Compute a deterministic socket/pipe address from namespace + id.
    fn socket_address(&self, id: &str) -> String {
        sys::socket_address(&self.args.address, &self.args.namespace, id)
    }

    /// Handle the `start` command: fork a child shim process, return socket address.
    pub fn start(&mut self) -> anyhow::Result<()> {
        log::info!("[{}] shim start", self.args.id);

        let address: String = self.socket_address(&self.args.id);

        // Fork: re-exec ourselves with the "run" action.
        // The child will create the socket/pipe and start the ttrpc server.
        let self_exe: PathBuf = std::env::current_exe()?;
        let cwd: PathBuf = std::env::current_dir()?;

        let mut cmd = std::process::Command::new(&self_exe);
        cmd.current_dir(&cwd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .arg("-namespace")
            .arg(&self.args.namespace)
            .arg("-id")
            .arg(&self.args.id)
            .arg("-address")
            .arg(&self.args.address)
            .arg("-publish-binary")
            .arg(&self.args.publish_binary)
            .arg("-socket")
            .arg(&address);

        if self.args.debug {
            cmd.arg("-debug");
        }

        let mut child = cmd.spawn()?;
        let pid: u32 = child.id();

        // Wait for the child to signal readiness by closing its stdout pipe.
        // The shimv2 protocol: the child calls signal_server_started() once the
        // ttrpc server is listening, which closes the write end of this pipe.
        if let Some(mut stdout) = child.stdout.take() {
            use std::io::Read;
            let mut buf: Vec<u8> = Vec::new();
            stdout.read_to_end(&mut buf)?;
        }

        // Write PID file and address file in the bundle directory.
        let bundle: PathBuf = if self.args.bundle.is_empty() {
            cwd.clone()
        } else {
            PathBuf::from(&self.args.bundle)
        };
        std::fs::write(bundle.join("shim.pid"), pid.to_string())?;
        std::fs::write(bundle.join("address"), &address)?;

        // Write address to stdout for containerd.
        std::io::stdout().write_all(address.as_bytes())?;
        std::io::stdout().flush()?;

        Ok(())
    }

    /// Handle the `delete` command: clean up resources, write exit info to stdout.
    pub async fn delete(&mut self) -> anyhow::Result<()> {
        log::info!("[{}] shim delete", self.args.id);

        // Remove the socket/pipe file.
        let address: String = self.socket_address(&self.args.id);
        let socket_path: &str = sys::parse_sockaddr(&address);
        if Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path).ok();
        }

        // Build and write a DeleteResponse protobuf to stdout.
        use protobuf::Message;
        let mut resp = containerd_shim_protos::api::DeleteResponse::new();
        resp.exit_status = 128 + 9; // SIGKILL
        let mut ts = protobuf::well_known_types::timestamp::Timestamp::new();
        ts.seconds = chrono::Utc::now().timestamp();
        resp.exited_at = Some(ts).into();

        let bytes: Vec<u8> = resp.write_to_bytes()?;
        std::io::stdout().write_all(&bytes)?;
        std::io::stdout().flush()?;

        Ok(())
    }

    /// Handle the `run` command: start ttrpc server with Task + Sandbox services.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        log::info!("[{}] shim run — starting ttrpc server", self.args.id);

        let runtime: Arc<dyn WorkloadRuntime> = self.runtime.clone();

        // Set up ttrpc server.
        let mut server = self.create_ttrpc_server(runtime.clone()).await?;

        server.start().await?;

        log::info!("[{}] ttrpc server started, signalling parent", self.args.id);

        // Signal parent that we're ready (platform-specific).
        sys::signal_server_started();

        log::info!("[{}] waiting for shutdown signal", self.args.id);

        // Wait for SIGTERM/SIGINT (Ctrl+C on Windows).
        tokio::signal::ctrl_c().await?;

        log::info!("[{}] shutting down", self.args.id);
        server.shutdown().await?;

        // Clean up socket/pipe file.
        if !self.args.socket.is_empty() {
            let socket_path: &str = sys::parse_sockaddr(&self.args.socket);
            if Path::new(socket_path).exists() {
                std::fs::remove_file(socket_path).ok();
            }
        }

        Ok(())
    }

    /// Create the ttrpc server with Task and Sandbox services registered.
    async fn create_ttrpc_server(
        &self,
        runtime: Arc<dyn WorkloadRuntime>,
    ) -> anyhow::Result<ttrpc::asynchronous::Server> {
        if self.args.socket.is_empty() {
            anyhow::bail!("no socket address provided");
        }

        // Create a platform-specific listener (Unix socket or Windows named pipe).
        let fd: i32 = sys::create_listener(&self.args.socket)?;

        // Build the ttrpc server from the listener.
        #[cfg(unix)]
        let server = unsafe { ttrpc::asynchronous::Server::new().add_unix_listener(fd)? };
        #[cfg(windows)]
        let server = {
            // On Windows, ttrpc creates the named pipe during start().
            // Pass the pipe address for the server to bind.
            ttrpc::asynchronous::Server::new()
        };

        // Register Task service.
        let task_svc: Arc<dyn shim_async::Task + Send + Sync> =
            Arc::new(NanvixTaskService::new(runtime.clone()));
        let server = server.register_service(shim_async::create_task(task_svc));

        // Register Sandbox service.
        let sandbox_svc: Arc<dyn sandbox_async::Sandbox + Send + Sync> =
            Arc::new(NanvixSandboxService::new(runtime));
        let server = server.register_service(sandbox_async::create_sandbox(sandbox_svc));

        Ok(server)
    }
}
