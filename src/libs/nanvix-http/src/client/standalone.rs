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
use ::log::{
    debug,
    error,
    info,
    trace,
};
use ::std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
};
use ::tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::UnixListener,
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
/// Configuration for standalone mode.
///
/// Holds the minimal set of paths required to launch a User VM without the full sandbox
/// cache infrastructure.
///
#[derive(Clone)]
pub struct StandaloneConfig {
    /// Path to the guest kernel binary.
    pub(crate) kernel_binary_path: String,
    /// Optional path to a RAM filesystem image exposed to the guest.
    pub(crate) ramfs_filename: Option<String>,
    /// Optional file path for capturing guest stderr output.
    pub(crate) console_file: Option<String>,
    /// Optional GDB server port for debugging the guest.
    #[cfg(feature = "gdb")]
    pub(crate) gdb_port: Option<u16>,
}

impl StandaloneConfig {
    ///
    /// # Description
    ///
    /// Creates a new standalone configuration.
    ///
    /// # Parameters
    ///
    /// - `kernel_binary_path`: Path to the guest kernel binary.
    /// - `ramfs_filename`: Optional path to a RAM filesystem image.
    /// - `console_file`: Optional file path for guest stderr capture.
    /// - `gdb_port`: Optional GDB server port.
    ///
    pub fn new(
        kernel_binary_path: String,
        ramfs_filename: Option<String>,
        console_file: Option<String>,
        #[cfg(feature = "gdb")] gdb_port: Option<u16>,
    ) -> Self {
        Self {
            kernel_binary_path,
            ramfs_filename,
            console_file,
            #[cfg(feature = "gdb")]
            gdb_port,
        }
    }
}

///
/// # Description
///
/// Bundles a running VM instance with its gateway bridge task and socket path.
///
struct RunningVm {
    /// Handle for the running VM.
    handle: StandaloneVmHandle,
    /// Task bridging the gateway Unix socket with the guest's IKC I/O channels.
    _gateway_bridge: JoinHandle<()>,
    /// Filesystem path of the gateway Unix socket (cleaned up on kill).
    gateway_socket_path: String,
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
            let _ = ::std::fs::remove_file(&vm.gateway_socket_path);
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
            state.config.kernel_binary_path.clone(),
            Some(message.program.clone()),
            initrd_args,
            state.config.ramfs_filename.clone(),
            state.config.console_file.clone(),
            None,
            #[cfg(feature = "gdb")]
            state.config.gdb_port,
        );

        // Create a Unix socket that serves as the gateway stream. The test harness (or any
        // consumer) connects to this socket to exchange I/O with the guest — exactly like the
        // multi-process gateway, but backed by IKC channels instead of a system VM.
        let gateway_socket_path: String =
            format!("/tmp/nvx-standalone-gw-{}.sock", std::process::id());
        let _ = ::std::fs::remove_file(&gateway_socket_path);
        let listener: UnixListener = match UnixListener::bind(&gateway_socket_path) {
            Ok(l) => l,
            Err(e) => {
                let reason: String =
                    format!("failed to bind gateway socket at {gateway_socket_path}: {e}");
                error!("serve_new(): {reason}");
                handle.abort();
                anyhow::bail!(reason);
            },
        };

        debug!("serve_new(): gateway socket bound at {gateway_socket_path}",);

        let gateway_bridge: JoinHandle<()> = tokio::spawn(gateway_bridge_task(listener, io));

        *guard = Some(RunningVm {
            handle,
            _gateway_bridge: gateway_bridge,
            gateway_socket_path: gateway_socket_path.clone(),
        });

        Ok(message::NewResponse {
            user_vm_id: STANDALONE_VM_ID,
            gateway_sockaddr: gateway_socket_path,
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
                running._gateway_bridge.abort();
                let _ = ::std::fs::remove_file(&running.gateway_socket_path);
                match running.handle.wait().await {
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

/// Size of the I/O buffer used by the gateway bridge for socket reads.
const GATEWAY_BRIDGE_BUFFER_SIZE: usize = 4096;

///
/// # Description
///
/// Bridges a Unix socket connection with the guest's IKC-based I/O channels.
///
/// Accepts exactly one connection on the listener, then runs a bidirectional relay:
/// - Socket reads → guest stdin (via `input_tx`)
/// - Guest stdout (via `output_rx`) → socket writes
///
/// The task exits when either the socket or the guest I/O channel closes.
///
/// # Parameters
///
/// - `listener`: Unix socket listener bound to the gateway path.
/// - `io`: I/O channels connected to the guest's stdin/stdout via IKC.
///
async fn gateway_bridge_task(listener: UnixListener, io: StandaloneVmIo) {
    let StandaloneVmIo {
        mut output_rx,
        input_tx,
    } = io;

    // Accept exactly one connection (the test harness or gateway consumer).
    let stream = match listener.accept().await {
        Ok((stream, _addr)) => {
            debug!("gateway_bridge_task(): accepted connection");
            stream
        },
        Err(e) => {
            error!("gateway_bridge_task(): failed to accept connection: {e}");
            return;
        },
    };

    let (mut reader, mut writer) = stream.into_split();

    // Spawn a task that reads from the socket and forwards to guest stdin.
    let input_handle: JoinHandle<()> = tokio::spawn(async move {
        let mut buffer: [u8; GATEWAY_BRIDGE_BUFFER_SIZE] = [0u8; GATEWAY_BRIDGE_BUFFER_SIZE];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => {
                    // EOF — consumer closed the write half.
                    break;
                },
                Ok(n) => {
                    if input_tx.send(buffer[..n].to_vec()).await.is_err() {
                        break;
                    }
                },
                Err(e) => {
                    trace!("gateway_bridge_task(): socket read error: {e}");
                    break;
                },
            }
        }
    });

    // Forward guest output to the socket writer.
    while let Some(data) = output_rx.recv().await {
        if let Err(e) = writer.write_all(&data).await {
            trace!("gateway_bridge_task(): socket write error: {e}");
            break;
        }
    }

    // Guest output channel closed — shut down the socket write half so the consumer sees EOF.
    let _ = writer.shutdown().await;

    // Wait for the input relay to finish.
    input_handle.abort();
    let _ = input_handle.await;

    debug!("gateway_bridge_task(): bridge closed");
}
