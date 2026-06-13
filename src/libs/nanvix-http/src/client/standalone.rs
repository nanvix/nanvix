// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Standalone deployment mode implementation for the HTTP client.
//!
//! In standalone mode, the HTTP client directly drives User VM instances without going through a
//! sandbox cache, system VM, control-plane, or gateway. Each NEW request spawns a VM with IKC-based
//! I/O channels, and each KILL request waits for the VM to finish and returns its exit code.
//! Guest stdout data is exposed via the I/O channels for gateway stream support.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message,
    message::{
        ErrorCode,
        MessageType,
        HTTP_HEADER_MESSAGE_TYPE,
    },
};
#[cfg(unix)]
use ::anyhow::Context;
use ::anyhow::Result;
use ::http_body_util::{
    BodyExt,
    Full,
};
use ::hyper::{
    body::{
        Bytes,
        Incoming,
    },
    service::Service,
    Request,
    Response,
    StatusCode,
};
#[cfg(unix)]
use ::log::warn;
use ::log::{
    debug,
    error,
    info,
    trace,
};
use ::nanvix_sandbox_config::StandaloneConfig;
use ::std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
};
#[cfg(windows)]
use ::tokio::net::windows::named_pipe::{
    NamedPipeServer,
    ServerOptions,
};
#[cfg(unix)]
use ::tokio::net::UnixListener;
use ::tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    sync::Mutex,
    task::JoinHandle,
};
use ::user_vm_api::UserVmIdentifier;
use ::uservm::standalone::{
    StandaloneVmHandle,
    StandaloneVmIo,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Bundles a running VM instance with its gateway bridge task and endpoint path.
///
struct RunningVm {
    /// Handle for the running VM.
    handle: StandaloneVmHandle,
    /// Task bridging the cross-platform gateway endpoint with the
    /// guest's IKC I/O channels.
    _gateway_bridge: JoinHandle<()>,
    /// Path of the gateway endpoint (UDS path on Unix, named pipe path
    /// on Windows). On Unix it is read at kill/cleanup to `unlink(2)`
    /// the underlying socket file; on Windows the named pipe is
    /// reclaimed automatically when the bridge task drops its server
    /// handle, so the field is only used to return the endpoint in
    /// the `NEW` response and is otherwise unread on that platform.
    #[cfg_attr(windows, allow(dead_code))]
    gateway_sockaddr: String,
}

///
/// # Description
///
/// Shared state for standalone mode, holding configuration and the single running VM.
///
/// In standalone mode at most one VM runs at a time.
///
pub struct StandaloneState {
    /// Configuration for launching new VMs.
    config: StandaloneConfig,
    /// The currently running VM instance with its I/O channels, if any.
    running_vm: Mutex<Option<RunningVm>>,
}

/// Fixed User VM identifier used in standalone mode (only one VM at a time).
const STANDALONE_VM_ID: UserVmIdentifier = UserVmIdentifier::new(1);

impl StandaloneState {
    ///
    /// # Description
    ///
    /// Creates a new standalone state with the given configuration.
    ///
    pub fn new(config: StandaloneConfig) -> Self {
        Self {
            config,
            running_vm: Mutex::new(None),
        }
    }

    ///
    /// # Description
    ///
    /// Returns whether a VM is currently running.
    ///
    pub async fn has_running_vm(&self) -> bool {
        self.running_vm.lock().await.is_some()
    }

    ///
    /// # Description
    ///
    /// Performs cleanup by aborting the running VM if one exists.
    ///
    pub async fn cleanup(&self) {
        if let Some(vm) = self.running_vm.lock().await.take() {
            info!("cleanup(): aborting VM");
            vm._gateway_bridge.abort();
            vm.handle.abort_and_wait().await;
            #[cfg(unix)]
            if let Err(e) = ::std::fs::remove_file(&vm.gateway_sockaddr) {
                if e.kind() != ::std::io::ErrorKind::NotFound {
                    warn!(
                        "cleanup(): failed to remove gateway socket {}: {e}",
                        vm.gateway_sockaddr
                    );
                }
            }
            debug!("cleanup(): VM cleaned up");
        }
    }
}

///
/// # Description
///
/// HTTP client handler for standalone mode.
///
/// This structure implements the Hyper Service trait to process incoming HTTP requests.
/// It directly drives User VM instances without going through a sandbox cache.
///
/// # Type Parameters
///
/// - `T`: Unused type parameter kept for API compatibility with other deployment modes.
///
pub(crate) struct HttpClient<T> {
    /// Shared standalone state holding configuration and running VMs.
    state: Arc<StandaloneState>,
    _phantom: PhantomData<T>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<T: Send + Sync + Default + 'static> super::HttpClient<T> {
    ///
    /// # Description
    ///
    /// Creates a new standalone HTTP client handler.
    ///
    /// # Parameters
    ///
    /// - `state`: Shared standalone state.
    ///
    /// # Returns
    ///
    /// A new HTTP client handler ready to process requests.
    ///
    pub(crate) fn new(state: Arc<StandaloneState>) -> Self {
        Self {
            state,
            _phantom: PhantomData,
        }
    }

    ///
    /// # Description
    ///
    /// Handles a NEW request by directly spawning a User VM with IKC-based I/O channels.
    ///
    /// Channels are created locally and the VM's stdout/stdin are bridged through the
    /// [`StandaloneVmIo`] channels. No system VM, control-plane, or gateway connections are
    /// established.
    ///
    /// # Parameters
    ///
    /// - `state`: Shared standalone state.
    /// - `message`: NEW message containing program and argument information.
    ///
    /// # Returns
    ///
    /// On success, returns a `NewResponse` containing the assigned User VM ID.
    /// On failure, returns an error describing what went wrong.
    ///
    pub(super) async fn serve_new(
        state: Arc<StandaloneState>,
        message: &message::New,
    ) -> Result<message::NewResponse> {
        trace!("serve_new(): {message:?}");

        let mut guard = state.running_vm.lock().await;
        if guard.is_some() {
            let reason: &str = "a VM is already running in standalone mode";
            error!("serve_new(): {reason}");
            anyhow::bail!(reason);
        }

        info!("serve_new(): spawning VM in standalone mode");

        let initrd_args: Option<String> = if message.program_args.is_empty() {
            None
        } else {
            Some(message.program_args.clone())
        };

        let (handle, io): (StandaloneVmHandle, StandaloneVmIo) = StandaloneVmHandle::spawn(
            state.config.kernel_binary_path().to_string(),
            Some(message.program.clone()),
            initrd_args,
            state.config.kernel_args().map(|s| s.to_string()),
            state.config.ramfs_filename().map(|s| s.to_string()),
            state.config.console_file().map(|s| s.to_string()),
            state.config.snapshot_path().map(|s| s.to_string()),
            state.config.mount_directory().map(|s| s.to_string()),
            state.config.networking_mode(),
            state.config.host_filter(),
            #[cfg(feature = "gdb")]
            state.config.gdb_port(),
        );

        // Cross-platform gateway endpoint: a single point at which a
        // host-side consumer (typically the containerd shim) connects to
        // exchange guest application stdio.
        //
        //   - Unix    : Unix-domain socket
        //   - Windows : Named pipe (`\\.\pipe\...`)
        //
        // The same `gateway_bridge_task` runs on both OSes; only the
        // binding primitive differs (see `bind_gateway_endpoint`).
        //
        // If the operator did not configure a path via `-gateway-sockaddr`,
        // we fall back to a per-process auto path so legacy consumers
        // (nanvix-bench / nanvix-terminal / integration tests) keep working
        // without any flag.
        let endpoint_path: String = match state.config.gateway_sockaddr() {
            Some(p) => p.to_string(),
            None => default_gateway_path(),
        };
        let endpoint: GatewayEndpoint = match bind_gateway_endpoint(&endpoint_path).await {
            Ok(ep) => ep,
            Err(e) => {
                let reason: String =
                    format!("failed to bind gateway endpoint at {endpoint_path}: {e}");
                error!("serve_new(): {reason}");
                handle.abort();
                anyhow::bail!(reason);
            },
        };
        debug!("serve_new(): gateway endpoint bound at {endpoint_path}");
        let gateway_bridge: JoinHandle<()> = tokio::spawn(gateway_bridge_task(endpoint, io));

        *guard = Some(RunningVm {
            handle,
            _gateway_bridge: gateway_bridge,
            gateway_sockaddr: endpoint_path.clone(),
        });

        Ok(message::NewResponse {
            user_vm_id: STANDALONE_VM_ID,
            gateway_sockaddr: endpoint_path,
        })
    }

    ///
    /// # Description
    ///
    /// Handles a KILL request by waiting for the specified VM to finish.
    ///
    /// The VM is removed from the running registry and its exit status is returned. The I/O
    /// handler task is also awaited to ensure clean shutdown.
    ///
    /// # Parameters
    ///
    /// - `state`: Shared standalone state.
    /// - `message`: KILL message containing the User VM identifier to terminate.
    ///
    /// # Returns
    ///
    /// On success, returns the VM's exit code. On failure, returns an error if the VM was not
    /// found or if the VM task panicked.
    ///
    pub(super) async fn serve_kill(
        state: Arc<StandaloneState>,
        _message: &message::Kill,
    ) -> Result<message::KillResponse> {
        let vm: Option<RunningVm> = state.running_vm.lock().await.take();
        match vm {
            Some(running) => {
                // Await VM exit first so the gateway bridge keeps draining
                // guest stdout/stderr until the guest itself terminates.
                // Aborting the bridge prematurely drops `output_rx`, which
                // causes the standalone I/O handler to return `-1` from
                // guest write() calls (see `uservm::standalone::
                // handle_write_request`), truncating logs and potentially
                // disrupting guest shutdown.
                let wait_result = running.handle.wait().await;
                // Defensive cleanup: the bridge typically exits on its own
                // once the guest closes its I/O channels, but abort it here
                // in case the host-side consumer is still connected.
                running._gateway_bridge.abort();
                #[cfg(unix)]
                if let Err(e) = ::std::fs::remove_file(&running.gateway_sockaddr) {
                    if e.kind() != ::std::io::ErrorKind::NotFound {
                        warn!(
                            "serve_kill(): failed to remove gateway socket {}: {e}",
                            running.gateway_sockaddr
                        );
                    }
                }
                match wait_result {
                    Ok(exit_status) => {
                        debug!("serve_kill(): VM exited (exit_status={exit_status})");
                        Ok(message::KillResponse {
                            exit_code: i32::from(exit_status),
                        })
                    },
                    Err(error) => {
                        error!("serve_kill(): VM failed (error={error:?})");
                        Ok(message::KillResponse { exit_code: -1 })
                    },
                }
            },
            None => {
                let reason: &str = "no VM is running in standalone mode";
                error!("serve_kill(): {reason}");
                Err(anyhow::anyhow!(reason))
            },
        }
    }
}

impl<T: Send + Sync + Default + 'static> Service<Request<Incoming>> for HttpClient<T> {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, request: Request<Incoming>) -> Self::Future {
        let state: Arc<StandaloneState> = self.state.clone();
        let future = async move {
            if Self::is_ready_request(&request) {
                return Ok(Self::ready_response());
            }

            // Get the request headers before consuming the body.
            let message_type: MessageType = match request
                .headers()
                .get(HTTP_HEADER_MESSAGE_TYPE)
                .and_then(|val| val.to_str().ok())
                .and_then(|s| s.parse::<MessageType>().ok())
            {
                Some(message_type) => message_type,
                None => {
                    let message: String =
                        format!("{} is a mandatory header", HTTP_HEADER_MESSAGE_TYPE);
                    error!("{message}");
                    return Ok(Self::error_response(
                        StatusCode::BAD_REQUEST,
                        ErrorCode::MissingMessageType,
                        message,
                    ));
                },
            };

            let body: Bytes = match request.collect().await {
                Ok(body) => body.to_bytes(),
                Err(_) => {
                    let reason: String = "failed to read body".to_string();
                    error!("{reason}");
                    return Ok(Self::error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ErrorCode::BodyReadFailed,
                        reason,
                    ));
                },
            };

            // Deserialize the request body and route to the corresponding function.
            let message_response: message::MessageResponse = match message_type {
                MessageType::New => {
                    debug!("deserializing NEW message with body: {body:?}");
                    let msg: message::New = match serde_json::from_slice(&body) {
                        Ok(msg) => msg,
                        Err(_) => {
                            let reason: String =
                                format!("failed to deserialize NEW message: {body:?}");
                            error!("{reason}");
                            return Ok(Self::error_response(
                                StatusCode::BAD_REQUEST,
                                ErrorCode::InvalidNewPayload,
                                reason,
                            ));
                        },
                    };

                    match Self::serve_new(state, &msg).await {
                        Ok(response) => message::MessageResponse::New(response),
                        Err(error) => {
                            let reason: String = format!("failed to process NEW request: {error}");
                            error!("{reason}");
                            return Ok(Self::error_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                ErrorCode::NewRequestFailed,
                                reason,
                            ));
                        },
                    }
                },
                MessageType::Kill => {
                    let msg: message::Kill = match serde_json::from_slice(&body) {
                        Ok(msg) => msg,
                        Err(e) => {
                            let reason: String = format!(
                                "failed to deserialize KILL message (error={e:?}): {body:?}"
                            );
                            error!("{reason}");
                            return Ok(Self::error_response(
                                StatusCode::BAD_REQUEST,
                                ErrorCode::InvalidKillPayload,
                                reason,
                            ));
                        },
                    };

                    debug!("serving KILL message:");
                    debug!("- user vm id: {}", msg.user_vm_id);

                    match Self::serve_kill(state, &msg).await {
                        Ok(response) => message::MessageResponse::Kill(response),
                        Err(error) => {
                            let reason: String = format!("failed to process KILL request: {error}");
                            error!("{reason}");
                            return Ok(Self::error_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                ErrorCode::KillRequestFailed,
                                reason,
                            ));
                        },
                    }
                },
            };

            Ok(Self::json_response(StatusCode::OK, &message_response))
        };
        Box::pin(future)
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Size of the I/O buffer used by the gateway bridge for reads from the
/// connected consumer.
const GATEWAY_BRIDGE_BUFFER_SIZE: usize = 4096;

///
/// # Description
///
/// Cross-platform listening primitive for the gateway endpoint. The
/// path determines the protocol: a `\\.\pipe\...` path on Windows yields
/// a named pipe server, any other path on Unix yields a UDS listener.
///
/// Wrapped in an enum so [`gateway_bridge_task`] is a single async
/// function on both OSes; only the accept step differs.
///
enum GatewayEndpoint {
    #[cfg(unix)]
    Unix(UnixListener),
    #[cfg(windows)]
    Pipe { server: NamedPipeServer },
}

#[cfg(unix)]
fn default_gateway_path() -> String {
    format!("/tmp/nvx-standalone-gw-{}.sock", std::process::id())
}

#[cfg(windows)]
fn default_gateway_path() -> String {
    format!(r"\\.\pipe\nanvix-standalone-gw-{}", std::process::id())
}

#[cfg(unix)]
async fn bind_gateway_endpoint(path: &str) -> ::anyhow::Result<GatewayEndpoint> {
    // Ensure parent directory exists (operator-supplied paths may point
    // into a per-sandbox state dir that we created earlier). Propagate
    // any failure here so the operator sees the real cause instead of a
    // misleading bind error downstream.
    if let Some(parent) = ::std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            ::std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create parent directory {} for gateway socket {path}",
                    parent.display()
                )
            })?;
        }
    }
    // Pre-bind cleanup: a stale socket file from a prior run is fine to
    // remove, but any other removal failure (e.g. EACCES, EISDIR) must
    // surface to the caller instead of being masked by the bind error.
    match ::std::fs::remove_file(path) {
        Ok(()) => {},
        Err(e) if e.kind() == ::std::io::ErrorKind::NotFound => {},
        Err(e) => {
            return Err(e)
                .with_context(|| format!("failed to remove stale gateway socket at {path}"));
        },
    }
    let listener = UnixListener::bind(path)?;
    // Restrict the gateway socket to the owning user. Without this the socket
    // inherits the process umask (typically 0755 / 0775) and any local user can
    // connect to the guest's stdin/stdout. This is in addition to per-sandbox
    // path isolation provided by the caller.
    {
        use ::std::os::unix::fs::PermissionsExt;
        let mut perms = ::std::fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        ::std::fs::set_permissions(path, perms)?;
    }
    Ok(GatewayEndpoint::Unix(listener))
}

#[cfg(windows)]
async fn bind_gateway_endpoint(path: &str) -> ::anyhow::Result<GatewayEndpoint> {
    let server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(path)?;
    Ok(GatewayEndpoint::Pipe { server })
}

///
/// # Description
///
/// Bridges the gateway endpoint with the guest's IKC-based I/O
/// channels. Accepts exactly one connection (the containerd shim, or a
/// test harness), then runs a bidirectional relay:
/// - Connection reads → guest stdin (via `input_tx`)
/// - Guest stdout/stderr (via `output_rx`) → connection writes
///
/// The task exits when either the connection or the guest I/O channel
/// closes. The implementation is identical on Unix and Windows; only the
/// accept primitive differs (UDS accept vs named pipe connect).
///
async fn gateway_bridge_task(endpoint: GatewayEndpoint, io: StandaloneVmIo) {
    let StandaloneVmIo {
        output_rx,
        input_tx,
    } = io;

    // Accept exactly one connection. On Unix this is a UDS accept; on
    // Windows we await the client connect on the pre-created pipe server.
    match endpoint {
        #[cfg(unix)]
        GatewayEndpoint::Unix(listener) => {
            let stream = match listener.accept().await {
                Ok((stream, _addr)) => {
                    debug!("gateway_bridge_task(): accepted UDS connection");
                    stream
                },
                Err(e) => {
                    error!("gateway_bridge_task(): failed to accept UDS connection: {e}");
                    return;
                },
            };
            let (reader, writer) = stream.into_split();
            run_bridge(reader, writer, output_rx, input_tx).await;
        },
        #[cfg(windows)]
        GatewayEndpoint::Pipe { server } => {
            if let Err(e) = server.connect().await {
                error!("gateway_bridge_task(): named pipe connect failed: {e}");
                // Drain output_rx so the guest doesn't see write() = -1.
                let mut output_rx = output_rx;
                while output_rx.recv().await.is_some() {}
                drop(input_tx);
                return;
            }
            debug!("gateway_bridge_task(): named pipe client connected");
            let (reader, writer) = tokio::io::split(server);
            run_bridge(reader, writer, output_rx, input_tx).await;
        },
    }

    debug!("gateway_bridge_task(): bridge closed");
}

/// Generic bidirectional pump used by both the Unix and Windows accept
/// paths. Reads from `reader` go to `input_tx` (guest stdin); writes to
/// `writer` come from `output_rx` (guest stdout/stderr).
async fn run_bridge<R, W>(
    mut reader: R,
    mut writer: W,
    mut output_rx: ::tokio::sync::mpsc::Receiver<Vec<u8>>,
    input_tx: ::tokio::sync::mpsc::Sender<Vec<u8>>,
) where
    R: ::tokio::io::AsyncRead + Unpin + Send + 'static,
    W: ::tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    // Spawn a task that reads from the connection and forwards to guest
    // stdin. This runs concurrently with the output direction below.
    let input_handle: JoinHandle<()> = tokio::spawn(async move {
        let mut buffer: [u8; GATEWAY_BRIDGE_BUFFER_SIZE] = [0u8; GATEWAY_BRIDGE_BUFFER_SIZE];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => {
                    if input_tx.send(buffer[..n].to_vec()).await.is_err() {
                        break;
                    }
                },
                Err(e) => {
                    trace!("gateway_bridge_task(): read error: {e}");
                    break;
                },
            }
        }
    });

    // Forward guest output to the connection.
    while let Some(data) = output_rx.recv().await {
        if let Err(e) = writer.write_all(&data).await {
            trace!("gateway_bridge_task(): write error: {e}");
            break;
        }
    }

    // Guest output channel closed — shut down the connection write half
    // so the consumer sees EOF, then unwind the input relay.
    let _ = writer.shutdown().await;
    input_handle.abort();
    let _ = input_handle.await;
}
