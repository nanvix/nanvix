//! Shim executor — handles the start/delete/run lifecycle commands.

use std::io::Write;
use std::os::unix::io::FromRawFd;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::args::ShimArgs;
use crate::sandbox_service::NanvixSandboxService;
use crate::task_service::NanvixTaskService;

use nanvix_shim_core::execution::ExecutionMode;
use nanvix_shim_core::registry::ModeRegistry;

const SOCKET_ROOT: &str = "/run/containerd";

/// The shim executor handles the shimv2 binary protocol commands.
pub struct ShimExecutor {
    pub args: ShimArgs,
    pub registry: ModeRegistry,
}

impl ShimExecutor {
    pub fn new(args: ShimArgs, registry: ModeRegistry) -> Self {
        Self { args, registry }
    }

    /// Compute a deterministic socket address from namespace + id.
    fn socket_address(&self, id: &str) -> String {
        let data = format!("{}/{}/{}", self.args.address, self.args.namespace, id);
        let hash = Sha256::digest(data.as_bytes());
        format!("unix://{}/s/{:x}", SOCKET_ROOT, hash)
    }

    /// Handle the `start` command: fork a child shim process, return socket address.
    pub fn start(&mut self) -> anyhow::Result<()> {
        log::info!("[{}] shim start", self.args.id);

        let address = self.socket_address(&self.args.id);

        // Fork: re-exec ourselves with the "run" action.
        // The child will create the socket and start the ttrpc server.
        let self_exe = std::env::current_exe()?;
        let cwd = std::env::current_dir()?;

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
        let pid = child.id();

        // Wait for the child to signal readiness by closing its stdout pipe.
        // The shimv2 protocol: the child does dup2(stderr, stdout) once the
        // ttrpc server is listening, which closes the write end of this pipe.
        if let Some(mut stdout) = child.stdout.take() {
            use std::io::Read;
            let mut buf = Vec::new();
            stdout.read_to_end(&mut buf)?;
        }

        // Write PID file and address file in the bundle directory
        let bundle = if self.args.bundle.is_empty() {
            cwd.clone()
        } else {
            PathBuf::from(&self.args.bundle)
        };
        std::fs::write(bundle.join("shim.pid"), pid.to_string())?;
        std::fs::write(bundle.join("address"), &address)?;

        // Write address to stdout for containerd
        std::io::stdout().write_all(address.as_bytes())?;
        std::io::stdout().flush()?;

        Ok(())
    }

    /// Handle the `delete` command: clean up resources, write exit info to stdout.
    pub async fn delete(&mut self) -> anyhow::Result<()> {
        log::info!("[{}] shim delete", self.args.id);

        // Remove the socket file
        let address = self.socket_address(&self.args.id);
        let socket_path = address.strip_prefix("unix://").unwrap_or(&address);
        if Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path).ok();
        }

        // Build and write a DeleteResponse protobuf to stdout
        use protobuf::Message;
        let mut resp = containerd_shim_protos::api::DeleteResponse::new();
        resp.exit_status = 128 + 9; // SIGKILL
        let mut ts = protobuf::well_known_types::timestamp::Timestamp::new();
        ts.seconds = chrono::Utc::now().timestamp();
        resp.exited_at = Some(ts).into();

        let bytes = resp.write_to_bytes()?;
        std::io::stdout().write_all(&bytes)?;
        std::io::stdout().flush()?;

        Ok(())
    }

    /// Handle the `run` command: start ttrpc server with Task + Sandbox services.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        log::info!("[{}] shim run — starting ttrpc server", self.args.id);

        // Create the execution mode from registry
        let runtime_config = nanvix_shim_core::config::NanvixRuntimeConfig::load_or_default();
        let mode = self.registry.create(
            &runtime_config.execution_mode,
            &self.args.id,
            &runtime_config,
        )?;

        // Set up ttrpc server
        let mut server = self.create_ttrpc_server(mode.clone()).await?;

        server.start().await?;

        log::info!("[{}] ttrpc server started, signalling parent", self.args.id);

        // Signal parent that we're ready by redirecting stdout to stderr.
        // The shimv2 protocol: parent blocks on reading child's stdout pipe;
        // dup2(stderr, stdout) closes the pipe end, parent gets EOF.
        // This is the same pattern used by containerd/rust-extensions.
        unsafe {
            libc::dup2(libc::STDERR_FILENO, libc::STDOUT_FILENO);
        }

        log::info!("[{}] waiting for shutdown signal", self.args.id);

        // Wait for SIGTERM/SIGINT
        tokio::signal::ctrl_c().await?;

        log::info!("[{}] shutting down", self.args.id);
        server.shutdown().await?;

        // Clean up socket file
        if !self.args.socket.is_empty() {
            let socket_path = self
                .args
                .socket
                .strip_prefix("unix://")
                .unwrap_or(&self.args.socket);
            if Path::new(socket_path).exists() {
                std::fs::remove_file(socket_path).ok();
            }
        }

        Ok(())
    }

    async fn create_ttrpc_server(
        &self,
        mode: Arc<dyn ExecutionMode>,
    ) -> anyhow::Result<ttrpc::asynchronous::Server> {
        use containerd_shim_protos::sandbox_async;
        use containerd_shim_protos::shim_async;

        if self.args.socket.is_empty() {
            anyhow::bail!("no socket address provided");
        }

        let socket_path = self
            .args
            .socket
            .strip_prefix("unix://")
            .unwrap_or(&self.args.socket);

        if let Some(parent) = Path::new(socket_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Remove stale socket if it exists
        if Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path)?;
        }

        let listener = UnixListener::bind(socket_path)?;
        use std::os::unix::io::IntoRawFd;
        let fd = listener.into_raw_fd();

        let server = unsafe {
            use std::os::unix::io::FromRawFd;
            ttrpc::asynchronous::Server::from_raw_fd(fd)
        };
        let server = server.set_domain_unix();

        // Register Task service
        let task_svc: Arc<Box<dyn shim_async::Task + Send + Sync>> =
            Arc::new(Box::new(NanvixTaskService::new(mode.clone())));
        let server = server.register_service(shim_async::create_task(task_svc));

        // Register Sandbox service
        let sandbox_svc: Arc<Box<dyn sandbox_async::Sandbox + Send + Sync>> =
            Arc::new(Box::new(NanvixSandboxService::new(mode)));
        let server = server.register_service(sandbox_async::create_sandbox(sandbox_svc));

        Ok(server)
    }
}
