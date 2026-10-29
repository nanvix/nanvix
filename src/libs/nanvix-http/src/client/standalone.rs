// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Standalone HTTP client implementation.
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
    /// Path of the gateway endpoint, held so that dropping it, along with the
    /// rest of the VM, removes the gateway socket file on Unix: the file goes
    /// away with the VM on every teardown path (see [`GatewayEndpointPath`]).
    _gateway_path: GatewayEndpointPath,
}

///
/// # Description
///
/// Path of the gateway endpoint bound for a running VM.
///
/// On Unix, the gateway endpoint is a Unix-domain socket, whose file outlives the listener that
/// bound it. Dropping this value removes the socket file, which ties the lifetime of the file to
/// the [`RunningVm`] that owns it: the file is removed on every path that tears the VM down,
/// including a `KILL` request that is cancelled before the VM exits. Only the socket file that was
/// bound is removed: if another file has replaced it at the same path, that file is left alone.
/// This check is best effort, as it cannot be made atomic with the removal, so a file that replaces
/// the socket right between the check and the removal is removed nonetheless. This only matters if
/// several daemons are configured with the same gateway path, which is not supported anyway, as
/// binding a gateway endpoint removes any file found at its path. On Windows, the gateway endpoint
/// is a named pipe, which the OS reclaims once its last handle is closed, so dropping this value
/// has no effect.
///
struct GatewayEndpointPath {
    /// Path at which the gateway endpoint is bound.
    path: String,
    /// Device and inode numbers of the socket file that was bound at `path`.
    #[cfg(unix)]
    file_id: (u64, u64),
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
    /// Performs cleanup by aborting the running VM if one exists, which removes its gateway
    /// socket.
    ///
    pub async fn cleanup(&self) {
        if let Some(vm) = self.running_vm.lock().await.take() {
            info!("cleanup(): aborting VM");
            vm.abort_and_wait().await;
            debug!("cleanup(): VM cleaned up");
        }
    }
}

impl RunningVm {
    ///
    /// # Description
    ///
    /// Waits for the VM to exit, and then tears it down.
    ///
    /// The gateway bridge keeps relaying guest output until the VM exits: aborting it earlier
    /// would drop `output_rx`, which makes the standalone I/O handler fail guest write() calls (see
    /// `uservm::standalone::handle_write_request`), truncating logs and potentially disrupting
    /// guest shutdown. The bridge typically exits on its own once the guest closes its I/O
    /// channels, but it is aborted once the VM has exited, in case the host-side consumer is still
    /// connected.
    ///
    /// This method takes ownership of the VM, so the path of its gateway endpoint goes out of scope
    /// when this method completes, which removes the gateway socket: the socket is gone by the time
    /// the caller learns that the VM exited. If this method is cancelled while it waits, the path
    /// goes out of scope along with the rest of the VM, so the socket is removed as well.
    ///
    /// # Returns
    ///
    /// On success, returns the exit status of the VM. On failure, returns an error describing why
    /// the VM failed.
    ///
    async fn wait(self) -> Result<u16> {
        let wait_result: Result<u16> = self.handle.wait().await;
        self._gateway_bridge.abort();
        wait_result
    }

    ///
    /// # Description
    ///
    /// Aborts the VM along with its gateway bridge, waits for the VM to settle, and then tears it
    /// down.
    ///
    /// This method takes ownership of the VM, so the path of its gateway endpoint goes out of scope
    /// when this method completes, which removes the gateway socket.
    ///
    async fn abort_and_wait(self) {
        self._gateway_bridge.abort();
        self.handle.abort_and_wait().await;
    }
}

impl GatewayEndpointPath {
    ///
    /// # Description
    ///
    /// Takes ownership of the gateway endpoint that was just bound at a given path.
    ///
    /// # Parameters
    ///
    /// - `path`: Path at which the gateway endpoint was bound.
    ///
    /// # Returns
    ///
    /// On success, returns the path of the gateway endpoint, which removes the endpoint from the
    /// filesystem when dropped on Unix. On failure, returns an error if the socket file of the
    /// endpoint cannot be inspected on Unix.
    ///
    fn new(path: &str) -> ::std::io::Result<Self> {
        #[cfg(unix)]
        let file_id: (u64, u64) = file_id(&::std::fs::symlink_metadata(path)?);
        Ok(Self {
            path: path.to_string(),
            #[cfg(unix)]
            file_id,
        })
    }

    ///
    /// # Description
    ///
    /// Returns the path at which the gateway endpoint is bound.
    ///
    fn as_str(&self) -> &str {
        self.path.as_str()
    }
}

#[cfg(unix)]
impl Drop for GatewayEndpointPath {
    fn drop(&mut self) {
        // Leave the path alone if it no longer refers to the socket file that was bound there.
        match ::std::fs::symlink_metadata(&self.path) {
            Ok(metadata) if file_id(&metadata) == self.file_id => {},
            Ok(_) => {
                warn!("drop(): gateway socket {} was replaced, leaving it in place", self.path);
                return;
            },
            Err(e) if e.kind() == ::std::io::ErrorKind::NotFound => return,
            Err(e) => {
                warn!("drop(): failed to inspect gateway socket {} (error={e})", self.path);
                return;
            },
        }
        match ::std::fs::remove_file(&self.path) {
            Ok(()) => debug!("drop(): removed gateway socket {}", self.path),
            Err(e) if e.kind() == ::std::io::ErrorKind::NotFound => {},
            Err(e) => warn!("drop(): failed to remove gateway socket {} (error={e})", self.path),
        }
    }
}

///
/// # Description
///
/// Returns the device and inode numbers that identify a file.
///
/// # Parameters
///
/// - `metadata`: Metadata of the file.
///
/// # Returns
///
/// The device and inode numbers of the file.
///
#[cfg(unix)]
fn file_id(metadata: &::std::fs::Metadata) -> (u64, u64) {
    use ::std::os::unix::fs::MetadataExt;
    (metadata.dev(), metadata.ino())
}

///
/// # Description
///
/// HTTP client handler for standalone mode.
///
/// This structure implements the Hyper Service trait to process incoming HTTP requests.
/// It directly drives User VM instances without going through a sandbox cache.
///
pub(crate) struct HttpClient {
    /// Shared standalone state holding configuration and running VMs.
    state: Arc<StandaloneState>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl super::HttpClient {
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
        Self { state }
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
        let (endpoint, gateway_path): (GatewayEndpoint, GatewayEndpointPath) =
            match bind_gateway_endpoint(&endpoint_path).await {
                Ok(bound) => bound,
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
        let gateway_sockaddr: String = gateway_path.as_str().to_string();

        *guard = Some(RunningVm {
            handle,
            _gateway_bridge: gateway_bridge,
            _gateway_path: gateway_path,
        });

        Ok(message::NewResponse {
            user_vm_id: STANDALONE_VM_ID,
            gateway_sockaddr,
        })
    }

    ///
    /// # Description
    ///
    /// Handles a KILL request by waiting for the specified VM to finish.
    ///
    /// The VM is removed from the running registry and its exit status is returned. The I/O
    /// handler task is also awaited to ensure clean shutdown, and the VM is torn down before its
    /// exit status is returned, which removes its gateway socket.
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
            Some(running) => match running.wait().await {
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
            },
            None => {
                let reason: &str = "no VM is running in standalone mode";
                error!("serve_kill(): {reason}");
                Err(anyhow::anyhow!(reason))
            },
        }
    }
}

impl Service<Request<Incoming>> for HttpClient {
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
/// connected consumer on Unix, where the input direction is a raw byte stream.
#[cfg(unix)]
const GATEWAY_BRIDGE_BUFFER_SIZE: usize = 4096;

/// Number of bytes in the length prefix that frames each Windows gateway-input record.
///
/// See [`forward_consumer_input`] for the framing rationale: it emulates the UDS half-close
/// that Windows named pipes lack so the guest still observes an isolated stdin EOF.
#[cfg(windows)]
const GATEWAY_INPUT_FRAME_HEADER_LEN: usize = ::std::mem::size_of::<u32>();

/// Maximum payload size (in bytes) accepted for a single Windows gateway-input record.
///
/// The length prefix is attacker-controlled, so it is capped to bound the per-record allocation
/// and prevent a malformed or malicious consumer from requesting an enormous frame (DoS). A frame
/// exceeding this limit aborts the input relay.
#[cfg(windows)]
const GATEWAY_INPUT_FRAME_MAX_PAYLOAD_LEN: usize = 1 << 20;

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
async fn bind_gateway_endpoint(
    path: &str,
) -> ::anyhow::Result<(GatewayEndpoint, GatewayEndpointPath)> {
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
    // The socket file exists from now on, so take ownership of it right
    // away: if any of the remaining steps fails, dropping the path removes
    // the file instead of leaking it.
    let gateway_path: GatewayEndpointPath = GatewayEndpointPath::new(path)?;
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
    Ok((GatewayEndpoint::Unix(listener), gateway_path))
}

#[cfg(windows)]
async fn bind_gateway_endpoint(
    path: &str,
) -> ::anyhow::Result<(GatewayEndpoint, GatewayEndpointPath)> {
    let server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(path)?;
    Ok((GatewayEndpoint::Pipe { server }, GatewayEndpointPath::new(path)?))
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
/// paths. Input records from `reader` go to `input_tx` (guest stdin);
/// writes to `writer` come from `output_rx` (guest stdout/stderr).
///
/// The input direction is decoded by [`forward_consumer_input`], whose wire
/// format differs per platform: a raw byte stream on Unix (EOF is the UDS
/// half-close) and a length-prefixed framing on Windows (EOF is an in-band
/// zero-length record, since named pipes have no half-close primitive).
async fn run_bridge<R, W>(
    reader: R,
    mut writer: W,
    mut output_rx: ::tokio::sync::mpsc::Receiver<Vec<u8>>,
    input_tx: ::tokio::sync::mpsc::Sender<Vec<u8>>,
) where
    R: ::tokio::io::AsyncRead + Unpin + Send + 'static,
    W: ::tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    // Spawn a task that reads from the connection and forwards to guest
    // stdin. This runs concurrently with the output direction below.
    let input_handle: JoinHandle<()> = tokio::spawn(forward_consumer_input(reader, input_tx));

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

/// Forwards consumer input to the guest's stdin channel on Unix, where the
/// gateway input direction is a raw byte stream. A zero-length read is the
/// transport EOF (a UDS `shutdown(SHUT_WR)` half-close); returning from this
/// function drops `input_tx`, which delivers EOF to the guest's stdin.
#[cfg(unix)]
async fn forward_consumer_input<R>(mut reader: R, input_tx: ::tokio::sync::mpsc::Sender<Vec<u8>>)
where
    R: ::tokio::io::AsyncRead + Unpin + Send + 'static,
{
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
}

/// Forwards consumer input to the guest's stdin channel on Windows, where the
/// gateway input direction is framed to emulate the UDS half-close that named
/// pipes lack.
///
/// Each record is a little-endian `u32` length followed by that many payload
/// bytes; a zero-length record is the in-band EOF marker. Decoding stops at the
/// EOF record (or when the pipe closes), and returning from this function drops
/// `input_tx`, which delivers EOF to the guest's stdin while the output
/// direction stays open. Because the records share the pipe's FIFO with the
/// preceding data, every stdin byte is guaranteed to reach the guest before the
/// EOF is observed — there is no data/control race.
#[cfg(windows)]
async fn forward_consumer_input<R>(mut reader: R, input_tx: ::tokio::sync::mpsc::Sender<Vec<u8>>)
where
    R: ::tokio::io::AsyncRead + Unpin + Send + 'static,
{
    loop {
        let mut header: [u8; GATEWAY_INPUT_FRAME_HEADER_LEN] =
            [0u8; GATEWAY_INPUT_FRAME_HEADER_LEN];
        if let Err(e) = reader.read_exact(&mut header).await {
            // A clean pipe close before the next record boundary is the consumer going away;
            // anything else is logged for diagnostics. Either way the guest's stdin is closed.
            if e.kind() != ::std::io::ErrorKind::UnexpectedEof {
                trace!("gateway_bridge_task(): input header read error: {e}");
            }
            break;
        }
        let payload_len: usize = u32::from_le_bytes(header) as usize;
        if payload_len == 0 {
            // In-band EOF record: the consumer finished writing stdin.
            break;
        }
        if payload_len > GATEWAY_INPUT_FRAME_MAX_PAYLOAD_LEN {
            // The length prefix is attacker-controlled; reject oversized frames to bound the
            // allocation below and avoid a memory-exhaustion DoS.
            trace!(
                "gateway_bridge_task(): input frame too large ({} > {} bytes), aborting relay",
                payload_len,
                GATEWAY_INPUT_FRAME_MAX_PAYLOAD_LEN
            );
            break;
        }
        let mut payload: Vec<u8> = vec![0u8; payload_len];
        if let Err(e) = reader.read_exact(&mut payload).await {
            trace!("gateway_bridge_task(): input payload read error: {e}");
            break;
        }
        if input_tx.send(payload).await.is_err() {
            break;
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use ::nanvix_sandbox_config::{
        HostFilter,
        NetworkingMode,
    };
    use ::std::path::PathBuf;
    use ::tempfile::TempDir;

    #[tokio::test]
    async fn gateway_endpoint_path_removes_socket_on_drop() {
        let tmp_dir: TempDir = TempDir::new().expect("failed to create temporary directory");
        let path: PathBuf = tmp_dir.path().join("gateway.sock");
        let path_str: &str = path.to_str().expect("temporary path is not valid UTF-8");
        let (endpoint, gateway_path): (GatewayEndpoint, GatewayEndpointPath) =
            bind_gateway_endpoint(path_str)
                .await
                .expect("failed to bind gateway endpoint");
        assert_eq!(gateway_path.as_str(), path_str);
        assert!(path.exists(), "gateway socket should exist once bound");

        // The socket file must go away with its path, even while the listener is still open.
        drop(gateway_path);
        assert!(!path.exists(), "gateway socket should be removed along with its path");
        drop(endpoint);
    }

    #[tokio::test]
    async fn gateway_endpoint_path_leaves_replaced_socket_alone() {
        let tmp_dir: TempDir = TempDir::new().expect("failed to create temporary directory");
        let path: PathBuf = tmp_dir.path().join("gateway.sock");
        let path_str: &str = path.to_str().expect("temporary path is not valid UTF-8");
        let (old_endpoint, old_gateway_path): (GatewayEndpoint, GatewayEndpointPath) =
            bind_gateway_endpoint(path_str)
                .await
                .expect("failed to bind gateway endpoint");
        // Another daemon that is configured with the same path binds its own gateway endpoint
        // there, which replaces the socket file.
        let (new_endpoint, new_gateway_path): (GatewayEndpoint, GatewayEndpointPath) =
            bind_gateway_endpoint(path_str)
                .await
                .expect("failed to bind replacing gateway endpoint");

        drop(old_gateway_path);
        assert!(path.exists(), "replacing gateway socket should be left alone");
        drop(new_gateway_path);
        assert!(!path.exists(), "gateway socket should be removed along with its path");
        drop(old_endpoint);
        drop(new_endpoint);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn serve_kill_removes_gateway_socket() {
        let tmp_dir: TempDir = TempDir::new().expect("failed to create temporary directory");
        let gateway_path: PathBuf = tmp_dir.path().join("gateway.sock");
        // VMs fail to boot right away, as their kernel does not exist.
        let config: StandaloneConfig = StandaloneConfig::new(
            tmp_dir
                .path()
                .join("missing-kernel.elf")
                .to_string_lossy()
                .into_owned(),
            None,
            None,
            None,
            None,
            None,
            NetworkingMode::Disabled,
            HostFilter::default(),
            #[cfg(feature = "gdb")]
            None,
            Some(gateway_path.to_string_lossy().into_owned()),
        );
        let state: Arc<StandaloneState> = Arc::new(StandaloneState::new(config));

        let new_message: message::New = message::New {
            tenant_id: "tenant".to_string(),
            app_name: "app".to_string(),
            program: tmp_dir
                .path()
                .join("program.elf")
                .to_string_lossy()
                .into_owned(),
            program_args: String::new(),
        };
        let new_response: message::NewResponse = HttpClient::serve_new(state.clone(), &new_message)
            .await
            .expect("failed to start VM");
        assert!(gateway_path.exists(), "gateway socket should exist while the VM is running");

        let kill_message: message::Kill = message::Kill {
            user_vm_id: new_response.user_vm_id,
        };
        let kill_response: message::KillResponse =
            HttpClient::serve_kill(state.clone(), &kill_message)
                .await
                .expect("failed to kill VM");
        assert_eq!(kill_response.exit_code, -1, "VM without a kernel should fail");
        // The reply to the KILL request is built from the returned response, so the socket must
        // be gone by the time the client learns that the VM exited.
        assert!(!gateway_path.exists(), "gateway socket should be removed when KILL completes");
        assert!(!state.has_running_vm().await, "VM should no longer be running");
    }
}
