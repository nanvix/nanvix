// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! HTTP (standalone) deployment mode for `nanvixd-vmm`.
//!
//! This mirrors the production `nanvixd` standalone HTTP contract so that
//! existing clients and the `nanvix-test` harness work against `nanvixd-vmm`
//! unchanged:
//!
//! - A tokio + hyper control server listens on `-http-addr <host:port>` and
//!   serves `POST /` requests selected by the `X-NVX-Message-Type` header:
//!   `NEW` (spawn a guest) and `KILL` (await its exit). A `GET /ready` probe
//!   returns `204 No Content`.
//! - Each `NEW` binds a per-VM **gateway Unix socket** whose path is returned in
//!   the response; the host-side consumer connects to it to exchange the guest's
//!   stdin/stdout, exactly as with the real `nanvixd`.
//!
//! The guest itself runs on the OpenVMM stack ([`crate::vmm::run`]) on a
//! dedicated thread driven by `pal_async`, while the control server and gateway
//! bridge run on the tokio runtime. The two are connected through the in-process
//! [`ChannelGuestIo`] endpoint: the gateway connection feeds the guest's stdin
//! and drains its stdout.

use crate::{
    build_guest_image,
    io::{
        ChannelGuestIo,
        GuestIoHandle,
    },
    open_console,
    vmm,
};
use ::anyhow::Context as _;
use ::http_body_util::{
    BodyExt as _,
    Full,
};
use ::hyper::{
    body::{
        Bytes,
        Incoming,
    },
    service::Service,
    Method,
    Request,
    Response,
    StatusCode,
};
use ::hyper_util::rt::TokioIo;
use ::log::{
    debug,
    error,
    info,
};
use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::{
    future::Future,
    os::unix::fs::PermissionsExt as _,
    path::{
        Path,
        PathBuf,
    },
    pin::Pin,
    sync::Arc,
    thread::JoinHandle,
};
use ::tokio::{
    io::{
        AsyncReadExt as _,
        AsyncWriteExt as _,
    },
    net::{
        TcpListener,
        UnixListener,
    },
    signal::unix::{
        signal,
        SignalKind,
    },
    sync::Mutex,
};
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Constants
//==================================================================================================

/// HTTP header that identifies the control message type (`NEW` / `KILL`).
const HTTP_HEADER_MESSAGE_TYPE: &str = "X-NVX-Message-Type";

/// Identifier reported for the single standalone VM (matches `nanvix-http`).
const STANDALONE_VM_ID: UserVmIdentifier = UserVmIdentifier::new(1);

/// Buffer size used when relaying gateway connection bytes to the guest.
const GATEWAY_BRIDGE_BUFFER_SIZE: usize = 64 * 1024;

//==================================================================================================
// Wire types (mirror `nanvix-http::message`)
//==================================================================================================

/// `NEW` request payload: the program to boot and its arguments.
#[derive(Debug, Deserialize)]
struct New {
    /// Tenant identifier (unused in standalone, accepted for compatibility).
    #[allow(dead_code)]
    #[serde(default)]
    tenant_id: String,
    /// Application name (unused in standalone, accepted for compatibility).
    #[allow(dead_code)]
    #[serde(default)]
    app_name: String,
    /// Path to the program (initrd) to boot.
    program: String,
    /// Command-line arguments forwarded to the program.
    #[serde(default)]
    program_args: String,
}

/// Response to a `NEW` request.
#[derive(Debug, Serialize)]
struct NewResponse {
    /// Identifier assigned to the spawned VM.
    user_vm_id: UserVmIdentifier,
    /// Path of the gateway Unix socket carrying the guest's stdio.
    gateway_sockaddr: String,
}

/// `KILL` request payload.
#[derive(Debug, Deserialize)]
struct Kill {
    /// Identifier of the VM to terminate (single VM in standalone).
    #[allow(dead_code)]
    user_vm_id: UserVmIdentifier,
}

/// Response to a `KILL` request.
#[derive(Debug, Serialize)]
struct KillResponse {
    /// Exit code of the guest workload (or `-1` on failure).
    exit_code: i32,
}

/// Structured error payload returned when a request cannot be fulfilled.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    /// Short machine-readable error code.
    code: &'static str,
    /// Human-readable diagnostic message.
    message: String,
}

//==================================================================================================
// Configuration and state
//==================================================================================================

/// Static configuration shared by every guest the server spawns.
pub struct HttpConfig {
    /// Directory containing `kernel.elf` and guest binaries.
    pub bin_dir: PathBuf,
    /// Optional RAM filesystem image.
    pub ramfs: Option<PathBuf>,
    /// Optional kernel arguments written to the control page.
    pub kernel_args: Option<String>,
    /// Optional file that receives the guest kernel console (default: stderr).
    pub console_file: Option<PathBuf>,
    /// Optional host directory served to the guest via `hostfsd`.
    pub mount_directory: Option<PathBuf>,
    /// Whether host networking is served via `networkd`.
    pub networking: bool,
    /// Guest RAM size in bytes.
    pub mem_size: u64,
}

/// A spawned guest and the resources bound to it.
struct RunningVm {
    /// Thread running the OpenVMM guest; joins to the guest exit code.
    vm_thread: JoinHandle<u16>,
    /// Task bridging the gateway socket and the guest's stdio.
    gateway_bridge: ::tokio::task::JoinHandle<()>,
    /// Filesystem path of the gateway socket (removed on teardown).
    gateway_path: PathBuf,
}

/// Shared server state: the static config and the single running VM (if any).
#[derive(Clone)]
struct State {
    /// Static guest configuration.
    config: Arc<HttpConfig>,
    /// The running VM, or `None` when idle. Standalone serves one VM at a time.
    running: Arc<Mutex<Option<RunningVm>>>,
}

//==================================================================================================
// Server
//==================================================================================================

/// Runs the HTTP control server on `sockaddr` until a shutdown signal.
///
/// Returns once `SIGINT` or `SIGTERM` is received, after tearing down any
/// running VM. The socket is bound before this returns control to the accept
/// loop so a client can use a successful TCP connect as a readiness signal.
pub async fn serve(sockaddr: &str, config: HttpConfig) -> ::anyhow::Result<()> {
    let state: State = State {
        config: Arc::new(config),
        running: Arc::new(Mutex::new(None)),
    };

    let listener: TcpListener = TcpListener::bind(sockaddr)
        .await
        .with_context(|| format!("failed to bind HTTP socket at {sockaddr}"))?;
    info!("nanvixd-vmm HTTP server listening on {sockaddr}");

    let mut sigint: ::tokio::signal::unix::Signal =
        signal(SignalKind::interrupt()).context("failed to install SIGINT handler")?;
    let mut sigterm: ::tokio::signal::unix::Signal =
        signal(SignalKind::terminate()).context("failed to install SIGTERM handler")?;

    loop {
        ::tokio::select! {
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, _addr)) => {
                        // Send small JSON control responses immediately.
                        if let Err(e) = stream.set_nodelay(true) {
                            error!("failed to set TCP_NODELAY: {e}");
                        }
                        let service: HttpService = HttpService { state: state.clone() };
                        let io: TokioIo<::tokio::net::TcpStream> = TokioIo::new(stream);
                        // Standalone serves one VM at a time, so connections are
                        // handled sequentially (the harness uses one connection
                        // per control request).
                        if let Err(e) = ::hyper::server::conn::http1::Builder::new()
                            .serve_connection(io, service)
                            .await
                        {
                            error!("failed to serve connection: {e}");
                        }
                    },
                    Err(e) => error!("failed to accept connection: {e}"),
                }
            },
            _ = sigint.recv() => {
                info!("received SIGINT, stopping nanvixd-vmm HTTP server");
                break;
            },
            _ = sigterm.recv() => {
                info!("received SIGTERM, stopping nanvixd-vmm HTTP server");
                break;
            },
        }
    }

    teardown(&state).await;
    Ok(())
}

/// Tears down any running VM on server shutdown.
async fn teardown(state: &State) {
    if let Some(vm) = state.running.lock().await.take() {
        vm.gateway_bridge.abort();
        remove_gateway_socket(&vm.gateway_path);
        // The guest thread is detached: it is reaped when the process exits.
    }
}

//==================================================================================================
// Request handling
//==================================================================================================

/// The hyper service: one instance per accepted connection.
#[derive(Clone)]
struct HttpService {
    /// Shared server state.
    state: State,
}

impl Service<Request<Incoming>> for HttpService {
    type Response = Response<Full<Bytes>>;
    type Error = ::hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, request: Request<Incoming>) -> Self::Future {
        let state: State = self.state.clone();
        Box::pin(async move { Ok(handle_request(state, request).await) })
    }
}

/// Dispatches a single HTTP request to the matching handler.
async fn handle_request(state: State, request: Request<Incoming>) -> Response<Full<Bytes>> {
    // Readiness probe.
    if request.method() == Method::GET && request.uri().path() == "/ready" {
        return empty_response(StatusCode::NO_CONTENT);
    }

    // Read the message-type header before consuming the body.
    let message_type: Option<String> = request
        .headers()
        .get(HTTP_HEADER_MESSAGE_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_ascii_lowercase());

    let body: Bytes = match request.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "BODY_READ_FAILED",
                "failed to read request body".to_string(),
            );
        },
    };

    match message_type.as_deref() {
        Some("new") => {
            let message: New = match serde_json::from_slice(&body) {
                Ok(message) => message,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "INVALID_NEW_PAYLOAD",
                        format!("failed to deserialize NEW message: {e}"),
                    );
                },
            };
            match serve_new(&state, &message).await {
                Ok(response) => json_response(StatusCode::OK, &response),
                Err(e) => {
                    error!("NEW request failed: {e:?}");
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "NEW_REQUEST_FAILED",
                        format!("failed to process NEW request: {e}"),
                    )
                },
            }
        },
        Some("kill") => {
            let message: Kill = match serde_json::from_slice(&body) {
                Ok(message) => message,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "INVALID_KILL_PAYLOAD",
                        format!("failed to deserialize KILL message: {e}"),
                    );
                },
            };
            match serve_kill(&state, &message).await {
                Ok(response) => json_response(StatusCode::OK, &response),
                Err(e) => {
                    error!("KILL request failed: {e:?}");
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "KILL_REQUEST_FAILED",
                        format!("failed to process KILL request: {e}"),
                    )
                },
            }
        },
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "MISSING_MESSAGE_TYPE",
            format!("{HTTP_HEADER_MESSAGE_TYPE} is a mandatory header"),
        ),
    }
}

/// Handles a `NEW` request: spawns a guest and binds its gateway socket.
async fn serve_new(state: &State, message: &New) -> ::anyhow::Result<NewResponse> {
    let mut guard = state.running.lock().await;
    if guard.is_some() {
        anyhow::bail!("a VM is already running in standalone mode");
    }

    let config: &HttpConfig = &state.config;
    let initrd_args: Option<String> = if message.program_args.is_empty() {
        None
    } else {
        Some(message.program_args.clone())
    };
    let image = build_guest_image(
        &config.bin_dir,
        Some(PathBuf::from(&message.program)),
        initrd_args,
        config.kernel_args.clone(),
        config.ramfs.clone(),
        config.mem_size,
    );
    let console = open_console(config.console_file.as_deref())
        .context("failed to open guest console sink")?;

    let (guest_io, handle): (ChannelGuestIo, GuestIoHandle) = ChannelGuestIo::pair();
    let mount_directory: Option<PathBuf> = config.mount_directory.clone();
    let networking: bool = config.networking;

    info!("serve_new(): spawning guest program={}", message.program);
    let vm_thread: JoinHandle<u16> = ::std::thread::Builder::new()
        .name("nanvixd-vmm-guest".to_string())
        .spawn(move || {
            let result = ::pal_async::DefaultPool::run_with(move |driver| async move {
                vmm::run(driver, image, Box::new(guest_io), console, mount_directory, networking)
                    .await
            });
            match result {
                Ok(code) => code,
                Err(e) => {
                    error!("guest run failed: {e:?}");
                    u16::MAX
                },
            }
        })
        .context("failed to spawn guest VM thread")?;

    let gateway_path: PathBuf = default_gateway_path();
    let listener: UnixListener = match bind_gateway(&gateway_path).await {
        Ok(listener) => listener,
        Err(e) => {
            // The guest thread is now running with no gateway; let it observe
            // EOF on stdin (the handle drops here) and reap it on process exit.
            return Err(e).context("failed to bind gateway socket");
        },
    };
    debug!("serve_new(): gateway bound at {}", gateway_path.display());
    let gateway_bridge = ::tokio::spawn(gateway_bridge_task(listener, handle));

    *guard = Some(RunningVm {
        vm_thread,
        gateway_bridge,
        gateway_path: gateway_path.clone(),
    });

    Ok(NewResponse {
        user_vm_id: STANDALONE_VM_ID,
        gateway_sockaddr: gateway_path.to_string_lossy().into_owned(),
    })
}

/// Handles a `KILL` request: awaits the guest's exit and returns its code.
async fn serve_kill(state: &State, _message: &Kill) -> ::anyhow::Result<KillResponse> {
    let vm: RunningVm = match state.running.lock().await.take() {
        Some(vm) => vm,
        None => anyhow::bail!("no VM is running in standalone mode"),
    };

    let RunningVm {
        vm_thread,
        gateway_bridge,
        gateway_path,
    } = vm;

    // The client closes the gateway write half (EOF to the guest's stdin) before
    // issuing KILL, so the guest exits on its own; join its thread to collect the
    // exit code. Joining blocks, so do it off the async runtime.
    let join_result = ::tokio::task::spawn_blocking(move || vm_thread.join()).await;

    // The guest has exited (or its thread panicked); the bridge can stop.
    gateway_bridge.abort();
    remove_gateway_socket(&gateway_path);

    let exit_code: i32 = match join_result {
        Ok(Ok(code)) => i32::from(code),
        Ok(Err(_)) => {
            error!("serve_kill(): guest thread panicked");
            -1
        },
        Err(e) => {
            error!("serve_kill(): failed to join guest thread: {e}");
            -1
        },
    };
    debug!("serve_kill(): guest exited (exit_code={exit_code})");
    Ok(KillResponse { exit_code })
}

//==================================================================================================
// Gateway bridge
//==================================================================================================

/// Bridges the gateway socket to the guest's in-process stdio endpoint.
///
/// Accepts exactly one connection, then relays bidirectionally:
/// - connection reads -> guest stdin (via [`crate::io::GuestStdinSender`]);
/// - guest stdout (via [`crate::io::GuestStdoutReceiver`]) -> connection writes.
///
/// The guest's output channel is drained on a blocking task because the
/// underlying receiver blocks the calling thread until the guest writes.
async fn gateway_bridge_task(listener: UnixListener, handle: GuestIoHandle) {
    let stream = match listener.accept().await {
        Ok((stream, _addr)) => stream,
        Err(e) => {
            error!("gateway_bridge_task(): accept failed: {e}");
            // Dropping `handle` signals EOF to the guest's stdin and closes its
            // stdout channel, so it does not block on a connection that never
            // arrives.
            return;
        },
    };
    debug!("gateway_bridge_task(): connection accepted");

    let (mut reader, mut writer) = stream.into_split();
    let (sender, receiver) = handle.split();

    // Connection -> guest stdin.
    let input_task = ::tokio::spawn(async move {
        let mut buffer: Vec<u8> = vec![0u8; GATEWAY_BRIDGE_BUFFER_SIZE];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => sender.send(&buffer[..n]),
                Err(e) => {
                    debug!("gateway_bridge_task(): read error: {e}");
                    break;
                },
            }
        }
        sender.close();
    });

    // Guest stdout -> connection. The blocking receiver is drained on a blocking
    // task that forwards to an async channel consumed below.
    let (tx, mut rx) = ::tokio::sync::mpsc::channel::<Vec<u8>>(64);
    let drain_task = ::tokio::task::spawn_blocking(move || {
        while let Some(data) = receiver.recv() {
            if tx.blocking_send(data).is_err() {
                break;
            }
        }
    });

    while let Some(data) = rx.recv().await {
        if let Err(e) = writer.write_all(&data).await {
            debug!("gateway_bridge_task(): write error: {e}");
            break;
        }
    }

    let _ = writer.shutdown().await;
    input_task.abort();
    drain_task.abort();
    debug!("gateway_bridge_task(): bridge closed");
}

//==================================================================================================
// Gateway socket helpers
//==================================================================================================

/// Returns a per-process default path for the gateway socket.
fn default_gateway_path() -> PathBuf {
    let mut path: PathBuf = ::std::env::temp_dir();
    path.push(format!("nanvixd-vmm-gw-{}.sock", ::std::process::id()));
    path
}

/// Binds the gateway Unix socket, restricting it to the owning user.
async fn bind_gateway(path: &Path) -> ::anyhow::Result<UnixListener> {
    match ::std::fs::remove_file(path) {
        Ok(()) => {},
        Err(e) if e.kind() == ::std::io::ErrorKind::NotFound => {},
        Err(e) => {
            return Err(e)
                .with_context(|| format!("failed to remove stale gateway socket at {path:?}"));
        },
    }
    let listener: UnixListener = UnixListener::bind(path)
        .with_context(|| format!("failed to bind gateway socket at {path:?}"))?;
    // Restrict the gateway socket to the owning user so no other local user can
    // attach to the guest's stdin/stdout.
    let mut perms = ::std::fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    ::std::fs::set_permissions(path, perms)?;
    Ok(listener)
}

/// Removes the gateway socket file, ignoring a missing file.
fn remove_gateway_socket(path: &Path) {
    if let Err(e) = ::std::fs::remove_file(path) {
        if e.kind() != ::std::io::ErrorKind::NotFound {
            error!("failed to remove gateway socket {path:?}: {e}");
        }
    }
}

//==================================================================================================
// Response helpers
//==================================================================================================

/// Builds a JSON response with the given status and serializable payload.
fn json_response<T: Serialize>(status: StatusCode, payload: &T) -> Response<Full<Bytes>> {
    match serde_json::to_vec(payload) {
        Ok(body) => Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap_or_else(|_| empty_response(StatusCode::INTERNAL_SERVER_ERROR)),
        Err(_) => empty_response(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Builds an empty response with the given status code.
fn empty_response(status: StatusCode) -> Response<Full<Bytes>> {
    let mut response: Response<Full<Bytes>> = Response::new(Full::new(Bytes::new()));
    *response.status_mut() = status;
    response
}

/// Builds a JSON error response.
fn error_response(
    status: StatusCode,
    code: &'static str,
    message: String,
) -> Response<Full<Bytes>> {
    json_response(status, &ErrorResponse { code, message })
}
