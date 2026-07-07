// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::nanvix_sandbox_config::HostFilter;
use ::networkd::NetworkDaemon;
use ::sys::ipc::{
    Message,
    MessageSender,
};
use ::syscall::SystemCallMessage;

#[cfg(target_os = "linux")]
use ::log::{
    error,
    info,
    warn,
};
#[cfg(target_os = "linux")]
use ::networkd::framing;
#[cfg(target_os = "linux")]
use ::networkd::wire::{
    NetworkOp,
    NetworkRequest,
    NetworkResponse,
    NetworkResult,
};
#[cfg(target_os = "linux")]
use ::std::{
    collections::HashMap,
    str::FromStr,
    sync::{
        Arc,
        Mutex,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
};
#[cfg(target_os = "linux")]
use ::sys::{
    error::ErrorCode,
    ipc::{
        MessageReceiver,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
#[cfg(target_os = "linux")]
use ::syscall::SystemCallMessageHeader;
#[cfg(target_os = "linux")]
use ::syscomm::{
    SocketStream,
    SocketStreamReader,
    SocketStreamWriter,
    SocketType,
    UnboundSocket,
    WriteAll,
};
#[cfg(target_os = "linux")]
use ::tokio::sync::{
    mpsc,
    oneshot,
};

//==================================================================================================
// NetworkTransport
//==================================================================================================

/// Abstracts where the network daemon runs.
///
/// Implementations handle standalone networking requests using host-owned buffers, whether the
/// daemon is in-process or reached through a local or remote process boundary.
pub trait NetworkTransport: Send + Sync {
    ///
    /// # Description
    ///
    /// Processes a single IKC message containing a networking system call request and returns
    /// the response message(s).
    ///
    /// # Parameters
    ///
    /// - `msg`: The incoming IKC message from the guest.
    ///
    /// # Returns
    ///
    /// On success, returns a vector of response messages to send back to the guest. On error (e.g.,
    /// unrecognized header), returns `None`.
    fn handle_message(&self, msg: Message) -> Option<Vec<Message>>;

    ///
    /// # Description
    ///
    /// Processes a `sendto()` request whose datagram payload was delivered out-of-band via a
    /// scatter/gather push.
    ///
    /// # Parameters
    ///
    /// - `source`: Identifies the calling thread.
    /// - `syscall_msg`: The parsed `SendToSocketRequest` system call message.
    /// - `data`: The datagram payload pulled from the caller.
    ///
    /// # Returns
    ///
    /// The response message to send back to the guest.
    fn handle_sendto(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
        data: &[u8],
    ) -> Message;

    ///
    /// # Description
    ///
    /// Processes a `recvfrom()` request whose datagram payload is delivered out-of-band via a
    /// scatter/gather pull.
    ///
    /// # Parameters
    ///
    /// - `source`: Identifies the calling thread.
    /// - `syscall_msg`: The parsed `ReceiveFromSocketRequest` system call message.
    ///
    /// # Returns
    ///
    /// A tuple with the response message and the datagram payload to push back to the guest.
    fn handle_recvfrom(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
    ) -> (Message, Vec<u8>);
}

//==================================================================================================
// LocalNetwork
//==================================================================================================

/// In-process [`NetworkTransport`] backed by a [`NetworkDaemon`].
pub struct LocalNetwork {
    daemon: NetworkDaemon,
}

impl LocalNetwork {
    ///
    /// # Description
    ///
    /// Creates a network daemon-backed transport.
    ///
    /// # Parameters
    ///
    /// - `filter`: The host egress filter applied to guest `connect()` destinations.
    ///
    /// # Returns
    ///
    /// On success, the new [`LocalNetwork`]. On failure, returns the underlying initialization
    /// error from [`NetworkDaemon`].
    ///
    pub fn new(filter: HostFilter) -> Result<Self> {
        Ok(Self {
            daemon: NetworkDaemon::new(filter)?,
        })
    }
}

impl NetworkTransport for LocalNetwork {
    fn handle_message(&self, msg: Message) -> Option<Vec<Message>> {
        self.daemon.handle_message(msg)
    }

    fn handle_sendto(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
        data: &[u8],
    ) -> Message {
        self.daemon.handle_sendto(source, syscall_msg, data)
    }

    fn handle_recvfrom(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
    ) -> (Message, Vec<u8>) {
        self.daemon.handle_recvfrom(source, syscall_msg)
    }
}

//==================================================================================================
// RemoteNetwork
//==================================================================================================

/// Capacity of the bounded channel that feeds encoded request frames to the writer task.
///
/// Bounding the channel turns a slow or stalled `networkd` into backpressure on the issuing guest
/// thread, rather than letting requests accumulate without limit.
#[cfg(target_os = "linux")]
const REQUEST_CHANNEL_CAPACITY: usize = 1024;

///
/// # Description
///
/// [`NetworkTransport`] implementation that forwards every request to a decoupled `networkd`
/// process over a socket.
///
/// The transport is deliberately thin: it carries only owned buffers (never guest-local memory
/// addresses), reproducing exactly what [`LocalNetwork`] hands to the in-process daemon. A single
/// background reader task and writer task multiplex all in-flight requests over one connection,
/// correlating responses with their originating request by the guest thread identifier (`tid`).
/// This works because a guest thread issues at most one blocking networking call at a time. It
/// keeps `networkd` itself byte-for-byte identical to standalone mode — only the wire between the
/// user VM and the daemon changes.
///
/// # Write semantics
///
/// Socket writes (`send`/`sendto`) round-trip exactly like standalone mode: the guest's outcome is
/// the response produced by `networkd` after it attempts the host-socket write. The client also
/// serializes writes per guest file descriptor before sending them to `networkd`, so only one write
/// for a socket can be outstanding on the daemon side at a time.
///
/// # Blocking contract
///
/// The [`NetworkTransport`] methods are synchronous and invoked from `spawn_blocking` workers, so
/// each call blocks the calling worker on a per-request channel until `networkd` responds. If the
/// connection is lost, blocked callers observe the failure and release their guest with an error so
/// no thread deadlocks.
///
#[cfg(target_os = "linux")]
pub struct RemoteNetwork {
    /// Requests awaiting a response, keyed by the originating guest thread identifier (raw `i32`).
    pending: Arc<Mutex<HashMap<i32, oneshot::Sender<NetworkResponse>>>>,
    /// Per-guest-fd locks that serialize socket writes before they are sent to `networkd`.
    write_locks: Arc<Mutex<HashMap<i32, Arc<Mutex<()>>>>>,
    /// Set once either transport task observes that the `networkd` connection is unusable.
    closed: Arc<AtomicBool>,
    /// Encoded request frames destined for the writer task.
    ///
    /// Bounded so that a slow `networkd` (or a stalled host-socket write) exerts backpressure: once
    /// the channel fills, [`blocking_send`](mpsc::Sender::blocking_send) blocks the calling worker,
    /// which in turn blocks the issuing guest thread.
    request_tx: mpsc::Sender<Vec<u8>>,
}

#[cfg(target_os = "linux")]
impl RemoteNetwork {
    ///
    /// # Description
    ///
    /// Connects to a decoupled `networkd` process and starts the background reader and writer
    /// tasks that service this transport.
    ///
    /// # Parameters
    ///
    /// - `sockaddr`: The socket address `networkd` is listening on.
    /// - `socket_type`: The socket address type (e.g. `unix` or `tcp`).
    ///
    /// # Returns
    ///
    /// On success, a connected [`RemoteNetwork`]. On failure, a human-readable error string.
    ///
    pub async fn connect(sockaddr: &str, socket_type: &str) -> Result<Self, String> {
        let socket_type: SocketType = SocketType::from_str(socket_type)
            .map_err(|e| format!("invalid networkd socket type: {e:?}"))?;
        let stream = UnboundSocket::new(socket_type)
            .connect(sockaddr)
            .await
            .map_err(|e| format!("failed to connect to networkd at {sockaddr}: {e:?}"))?;

        Ok(Self::from_stream(stream))
    }

    ///
    /// # Description
    ///
    /// Builds a transport over an already-connected `stream` to `networkd`, starting the background
    /// reader and writer tasks that service it.
    ///
    /// This is the shared core of [`RemoteNetwork::connect`]; it is factored out so the transport
    /// can also be driven over an in-memory socket pair in tests, without a real `networkd`
    /// listening on a socket address.
    ///
    /// # Parameters
    ///
    /// - `stream`: The already-connected transport to `networkd`.
    ///
    /// # Returns
    ///
    /// A [`RemoteNetwork`] whose reader and writer tasks are already running.
    ///
    fn from_stream(stream: SocketStream) -> Self {
        let (reader, writer): (SocketStreamReader, SocketStreamWriter) = stream.split();
        let pending: Arc<Mutex<HashMap<i32, oneshot::Sender<NetworkResponse>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let write_locks: Arc<Mutex<HashMap<i32, Arc<Mutex<()>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let closed: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let (request_tx, request_rx) = mpsc::channel::<Vec<u8>>(REQUEST_CHANNEL_CAPACITY);

        ::tokio::spawn(run_writer(writer, request_rx, pending.clone(), closed.clone()));
        ::tokio::spawn(run_reader(reader, pending.clone(), closed.clone()));

        Self {
            pending,
            write_locks,
            closed,
            request_tx,
        }
    }

    ///
    /// # Description
    ///
    /// Sends a request to `networkd` and blocks until the matching response arrives.
    ///
    /// # Parameters
    ///
    /// - `op`: The operation to forward.
    ///
    /// # Returns
    ///
    /// The response result on success, or `None` if the connection was lost before a response was
    /// received.
    ///
    fn round_trip(&self, op: NetworkOp) -> Option<NetworkResult> {
        let key: i32 = op_correlation_key(&op);
        let (response_tx, response_rx) = oneshot::channel::<NetworkResponse>();

        // Register the pending entry before sending so the reader can never deliver a response
        // before we are ready to receive it. A guest thread issues at most one blocking networking
        // call at a time, so its `tid` uniquely identifies this in-flight request.
        match self.pending.lock() {
            Ok(mut pending) => {
                if self.closed.load(Ordering::Acquire) {
                    return None;
                }
                pending.insert(key, response_tx);
            },
            Err(_) => return None,
        }

        let frame: Vec<u8> = match (NetworkRequest { op }.to_frame()) {
            Ok(frame) => frame,
            Err(e) => {
                // The request cannot be encoded (payload exceeds the wire limits); it can never be
                // serviced, so drop the pending entry and report failure to the caller.
                error!("networkd remote transport: failed to encode request frame: {e}");
                if let Ok(mut pending) = self.pending.lock() {
                    pending.remove(&key);
                }
                return None;
            },
        };
        if self.request_tx.blocking_send(frame).is_err() {
            // The writer task is gone; drop the now-unserviceable pending entry.
            if let Ok(mut pending) = self.pending.lock() {
                pending.remove(&key);
            }
            return None;
        }

        // Block this worker until the reader delivers our response or the connection drops (which
        // drops our sender, waking the receiver with an error).
        response_rx
            .blocking_recv()
            .ok()
            .map(|response| response.result)
    }

    ///
    /// # Description
    ///
    /// Returns the per-fd write serialization lock for `guest_fd`, creating it on first use.
    ///
    /// # Parameters
    ///
    /// - `guest_fd`: The guest-visible socket descriptor being written.
    ///
    fn write_lock(&self, guest_fd: i32) -> Option<Arc<Mutex<()>>> {
        match self.write_locks.lock() {
            Ok(mut locks) => Some(
                locks
                    .entry(guest_fd)
                    .or_insert_with(|| Arc::new(Mutex::new(())))
                    .clone(),
            ),
            Err(_) => None,
        }
    }
}

#[cfg(target_os = "linux")]
impl NetworkTransport for RemoteNetwork {
    fn handle_message(&self, msg: Message) -> Option<Vec<Message>> {
        let tid: ThreadIdentifier = extract_tid(msg.source);

        // Decode the syscall header only to recognize `send`, which must be serialized per socket
        // before it is sent to `networkd`. Messages that cannot be decoded simply round-trip.
        let syscall_msg: Option<SystemCallMessage> =
            SystemCallMessage::try_from_bytes(msg.payload).ok();
        let write_lock: Option<Arc<Mutex<()>>> = match syscall_msg.as_ref() {
            Some(syscall_msg) => {
                let header: SystemCallMessageHeader = syscall_msg.header;
                if header != SystemCallMessageHeader::SendSocketRequest {
                    None
                } else {
                    match syscall_target_fd(syscall_msg).and_then(|fd| self.write_lock(fd)) {
                        Some(lock) => Some(lock),
                        None => return Some(vec![remote_error(tid, ErrorCode::ConnectionAborted)]),
                    }
                }
            },
            _ => None,
        };
        let _write_guard = match write_lock.as_ref().map(|lock| lock.lock()) {
            Some(Ok(guard)) => Some(guard),
            Some(Err(_)) => return Some(vec![remote_error(tid, ErrorCode::ConnectionAborted)]),
            None => None,
        };

        match self.round_trip(NetworkOp::Message(msg)) {
            // `networkd` may legitimately have no response for an inline message; forward whatever
            // it produced verbatim.
            Some(NetworkResult::Message(messages)) => messages,
            // A wrong-variant or lost-connection outcome releases the guest with an error so it is
            // not left blocked forever.
            _ => Some(vec![remote_error(tid, ErrorCode::ConnectionAborted)]),
        }
    }

    fn handle_sendto(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
        data: &[u8],
    ) -> Message {
        let tid: ThreadIdentifier = extract_tid(source);

        let write_lock: Arc<Mutex<()>> =
            match syscall_target_fd(&syscall_msg).and_then(|fd| self.write_lock(fd)) {
                Some(lock) => lock,
                None => return remote_error(tid, ErrorCode::ConnectionAborted),
            };
        let _write_guard = match write_lock.lock() {
            Ok(guard) => guard,
            Err(_) => return remote_error(tid, ErrorCode::ConnectionAborted),
        };

        let msg: Message = rebuild_request_message(source, syscall_msg);
        match self.round_trip(NetworkOp::SendTo {
            msg,
            data: data.to_vec(),
        }) {
            Some(NetworkResult::SendTo(msg)) => msg,
            _ => remote_error(tid, ErrorCode::ConnectionAborted),
        }
    }

    fn handle_recvfrom(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
    ) -> (Message, Vec<u8>) {
        let tid: ThreadIdentifier = extract_tid(source);

        let msg: Message = rebuild_request_message(source, syscall_msg);
        match self.round_trip(NetworkOp::RecvFrom(msg)) {
            Some(NetworkResult::RecvFrom { msg, data }) => (msg, data),
            // Release the guest's pull with an empty transfer plus an error so it does not deadlock.
            _ => (remote_error(tid, ErrorCode::ConnectionAborted), Vec::new()),
        }
    }
}

//==================================================================================================
// Private Functions
//==================================================================================================

///
/// # Description
///
/// Rebuilds the request [`Message`] that carries `source` and the encoded `syscall_msg` across the
/// wire, exactly as the standalone I/O handler received it from the guest.
///
#[cfg(target_os = "linux")]
fn rebuild_request_message(source: MessageSender, syscall_msg: SystemCallMessage) -> Message {
    Message::new(
        source,
        MessageReceiver::NETWORKD,
        MessageType::Ikc,
        None,
        syscall_msg.into_bytes(),
    )
}

///
/// # Description
///
/// Extracts the originating thread identifier from a message source, falling back to a sentinel
/// thread when it is unset, mirroring the standalone I/O handler.
///
#[cfg(target_os = "linux")]
fn extract_tid(source: MessageSender) -> ThreadIdentifier {
    let tid: ThreadIdentifier = source.tid;
    if tid.is_none() {
        warn!("networkd client: message source has no thread id");
        return ThreadIdentifier::from(1i32);
    }
    tid
}

///
/// # Description
///
/// Derives the correlation key for a request from the guest thread identifier embedded in its
/// message source.
///
/// The key is the raw `i32` form of the originating `tid`. `networkd` echoes the same `tid` on the
/// response, so client and server agree on the key without any explicit request counter. A guest
/// thread issues at most one blocking networking call at a time, so its `tid` is unique among
/// in-flight requests.
///
/// # Parameters
///
/// - `op`: The operation whose correlation key is required.
///
/// # Returns
///
/// The correlation key.
///
#[cfg(target_os = "linux")]
fn op_correlation_key(op: &NetworkOp) -> i32 {
    let tid: ThreadIdentifier = match op {
        NetworkOp::Message(msg) | NetworkOp::RecvFrom(msg) => msg.source.tid,
        NetworkOp::SendTo { msg, .. } => msg.source.tid,
    };
    i32::from(tid)
}

///
/// # Description
///
/// Builds an error response [`Message`] addressed to `tid`, used to release a guest blocked on a
/// request that `networkd` could not service.
///
#[cfg(target_os = "linux")]
fn remote_error(tid: ThreadIdentifier, error: ErrorCode) -> Message {
    Message::new(
        MessageSender::NETWORKD,
        MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
        MessageType::Ikc,
        Some(error),
        [0u8; Message::PAYLOAD_SIZE],
    )
}

///
/// # Description
///
/// Returns the guest file descriptor a socket system call targets, if it carries one.
///
/// This is used to serialize remote writes per socket. Requests that create a descriptor
/// (`socket`, `socketpair`) target none and yield `None`, as do messages that do not decode to a
/// known socket request.
///
/// # Parameters
///
/// - `syscall_msg`: The decoded socket system-call message.
///
/// # Returns
///
/// `Some(fd)` with the target guest descriptor, or `None` if the call carries none.
///
#[cfg(target_os = "linux")]
fn syscall_target_fd(syscall_msg: &SystemCallMessage) -> Option<i32> {
    use ::syscall::{
        sys::socket::message::{
            AcceptSocketRequest,
            BindSocketRequest,
            ConnectSocketRequest,
            GetPeerNameRequest,
            GetSockNameRequest,
            ListenSocketRequest,
            ReceiveFromSocketRequest,
            ReceiveSocketRequest,
            SendSocketRequest,
            SendToSocketRequest,
            ShutdownSocketRequest,
        },
        unistd::message::CloseRequest,
    };

    let payload: [u8; SystemCallMessage::PAYLOAD_SIZE] = syscall_msg.payload;
    match syscall_msg.header {
        SystemCallMessageHeader::BindSocketRequest => {
            Some(BindSocketRequest::from_bytes(payload).sockfd)
        },
        SystemCallMessageHeader::ConnectSocketRequest => {
            Some(ConnectSocketRequest::from_bytes(payload).sockfd)
        },
        SystemCallMessageHeader::ListenSocketRequest => {
            Some(ListenSocketRequest::from_bytes(payload).sockfd)
        },
        SystemCallMessageHeader::AcceptSocketRequest => {
            Some(AcceptSocketRequest::from_bytes(payload).sockfd)
        },
        SystemCallMessageHeader::GetSockNameRequest => {
            Some(GetSockNameRequest::from_bytes(payload).sockfd)
        },
        SystemCallMessageHeader::GetPeerNameRequest => {
            Some(GetPeerNameRequest::from_bytes(payload).sockfd)
        },
        SystemCallMessageHeader::ShutdownSocketRequest => {
            Some(ShutdownSocketRequest::from_bytes(payload).sockfd)
        },
        SystemCallMessageHeader::ReceiveSocketRequest => {
            Some(ReceiveSocketRequest::from_bytes(payload).sockfd)
        },
        SystemCallMessageHeader::SendSocketRequest => {
            Some(SendSocketRequest::from_bytes(payload).sockfd)
        },
        SystemCallMessageHeader::ReceiveFromSocketRequest => {
            Some(ReceiveFromSocketRequest::from_bytes(payload).sockfd)
        },
        SystemCallMessageHeader::SendToSocketRequest => {
            Some(SendToSocketRequest::from_bytes(payload).sockfd)
        },
        SystemCallMessageHeader::CloseRequest => Some(CloseRequest::from_bytes(payload).fd),
        _ => None,
    }
}

///
/// # Description
///
/// Drains encoded request frames and writes them to `networkd`, terminating when every sender has
/// been dropped or a write fails.
///
#[cfg(target_os = "linux")]
async fn run_writer(
    mut writer: SocketStreamWriter,
    mut request_rx: mpsc::Receiver<Vec<u8>>,
    pending: Arc<Mutex<HashMap<i32, oneshot::Sender<NetworkResponse>>>>,
    closed: Arc<AtomicBool>,
) {
    while let Some(frame) = request_rx.recv().await {
        if let Err(e) = writer.write_all(&frame).await {
            error!("networkd client: failed to write request frame: {e}");
            close_connection(&pending, &closed);
            break;
        }
    }
}

///
/// # Description
///
/// Reads server frames from `networkd` and routes each correlated response to the waiting caller via
/// the pending map. On disconnect, every outstanding pending entry is dropped so blocked callers
/// observe the failure and release their guest.
///
#[cfg(target_os = "linux")]
async fn run_reader(
    mut reader: SocketStreamReader,
    pending: Arc<Mutex<HashMap<i32, oneshot::Sender<NetworkResponse>>>>,
    closed: Arc<AtomicBool>,
) {
    loop {
        let body: Vec<u8> = match framing::read_frame(&mut reader).await {
            Ok(Some(body)) => body,
            Ok(None) => {
                info!("networkd client: networkd disconnected");
                break;
            },
            Err(e) => {
                error!("networkd client: failed to read response frame: {e}");
                break;
            },
        };

        let response: NetworkResponse = match NetworkResponse::decode(&body) {
            Ok(response) => response,
            Err(e) => {
                error!("networkd client: malformed response frame: {e}");
                break;
            },
        };

        let key: i32 = i32::from(response.tid);
        let waiter: Option<oneshot::Sender<NetworkResponse>> = match pending.lock() {
            Ok(mut pending) => pending.remove(&key),
            Err(_) => {
                error!("networkd client: pending map poisoned");
                break;
            },
        };

        match waiter {
            // If the receiver was already dropped, the caller gave up; discarding is harmless.
            Some(waiter) => {
                let _ = waiter.send(response);
            },
            None => warn!("networkd client: response for unknown tid {key}"),
        }
    }

    // Dropping the outstanding senders wakes every blocked caller with an error.
    close_connection(&pending, &closed);
}

#[cfg(target_os = "linux")]
fn close_connection(
    pending: &Arc<Mutex<HashMap<i32, oneshot::Sender<NetworkResponse>>>>,
    closed: &AtomicBool,
) {
    closed.store(true, Ordering::Release);
    if let Ok(mut pending) = pending.lock() {
        pending.clear();
    } else {
        error!("networkd client: pending map poisoned while closing connection");
    }
}

//==================================================================================================
// Tests
//==================================================================================================

// The remote transport is Linux-only, so the integration tests that drive it are too.
#[cfg(all(test, target_os = "linux"))]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ::tokio::{
        net::UnixStream,
        task::JoinHandle,
        time::{
            Duration,
            timeout,
        },
    };

    /// Maximum time any single test is allowed to run before it is considered hung. A blocked guest
    /// that is never released would trip this instead of deadlocking the test binary.
    const TEST_TIMEOUT: Duration = Duration::from_secs(5);

    /// A payload large enough to exercise multi-kilobyte bulk transfers across the wire.
    const BULK_PAYLOAD_LEN: usize = 4096;

    /// Creates a pair of connected [`SocketStream`] instances backed by Unix sockets.
    fn unix_stream_pair() -> (SocketStream, SocketStream) {
        let (a, b): (UnixStream, UnixStream) =
            UnixStream::pair().expect("failed to create unix stream pair");
        (SocketStream::Unix(a), SocketStream::Unix(b))
    }

    /// Builds an owned networking [`Message`] whose first payload byte carries `marker`, used to
    /// distinguish messages when checking that responses are correlated to the right request. The
    /// message source carries a `tid` derived from `marker` so that distinct markers yield distinct
    /// correlation keys, exactly as distinct guest threads would.
    fn message_with_marker(marker: u8) -> Message {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0; Message::PAYLOAD_SIZE];
        payload[0] = marker;
        Message::new(
            MessageSender::new(
                ProcessIdentifier::NETWORKD,
                ThreadIdentifier::from(i32::from(marker)),
            ),
            MessageReceiver::NETWORKD,
            MessageType::Ikc,
            None,
            payload,
        )
    }

    /// Returns the guest thread identifier that a response must echo to correlate with `request`,
    /// mirroring the server's own correlation logic.
    fn request_tid(request: &NetworkRequest) -> ThreadIdentifier {
        match &request.op {
            NetworkOp::Message(msg) | NetworkOp::RecvFrom(msg) => msg.source.tid,
            NetworkOp::SendTo { msg, .. } => msg.source.tid,
        }
    }

    /// Builds a [`Message`] whose payload is not a valid system-call message, so the real
    /// [`NetworkDaemon`] rejects it without touching the networking backend.
    fn unparsable_message() -> Message {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0; Message::PAYLOAD_SIZE];
        payload[0] = 0xFF;
        payload[1] = 0xFF;
        Message::new(
            MessageSender::NETWORKD,
            MessageReceiver::NETWORKD,
            MessageType::Ikc,
            None,
            payload,
        )
    }

    /// Reads a single length-prefixed frame body from the server side of the wire, reusing the same
    /// shared framing codec the production client and server use. Returns `None` at a clean frame
    /// boundary or on any transport error, which is all these tests need to drive the responder.
    async fn read_frame_body(reader: &mut SocketStreamReader) -> Option<Vec<u8>> {
        framing::read_frame(reader).await.ok().flatten()
    }

    /// Spawns a stand-in `networkd` server that answers each request frame with the response
    /// produced by `respond`, returning `None` to drop a request without replying.
    fn spawn_responder<F>(stream: SocketStream, mut respond: F) -> JoinHandle<()>
    where
        F: FnMut(NetworkRequest) -> Option<NetworkResponse> + Send + 'static,
    {
        ::tokio::spawn(async move {
            let (mut reader, mut writer): (SocketStreamReader, SocketStreamWriter) = stream.split();
            while let Some(body) = read_frame_body(&mut reader).await {
                let request: NetworkRequest =
                    NetworkRequest::decode(&body).expect("responder: decode request");
                if let Some(response) = respond(request) {
                    writer
                        .write_all(&response.to_frame().expect("responder: encode response"))
                        .await
                        .expect("responder: write response");
                }
            }
        })
    }

    /// Runs a single blocking `round_trip` on `net` from a worker thread, mirroring how the demux
    /// invokes the transport from `spawn_blocking`.
    async fn round_trip_blocking(net: Arc<RemoteNetwork>, op: NetworkOp) -> Option<NetworkResult> {
        ::tokio::task::spawn_blocking(move || net.round_trip(op))
            .await
            .expect("blocking round_trip task")
    }

    /// An inline `Message` request round-trips and its response messages are delivered verbatim.
    #[tokio::test]
    async fn remote_message_round_trips() {
        let result: ::anyhow::Result<()> = timeout(TEST_TIMEOUT, async {
            let (client_end, server_end): (SocketStream, SocketStream) = unix_stream_pair();
            let _server: JoinHandle<()> = spawn_responder(server_end, |request| {
                let tid: ThreadIdentifier = request_tid(&request);
                Some(NetworkResponse {
                    tid,
                    result: NetworkResult::Message(Some(vec![message_with_marker(0x5A)])),
                })
            });

            let net: Arc<RemoteNetwork> = Arc::new(RemoteNetwork::from_stream(client_end));
            let outcome: Option<NetworkResult> =
                round_trip_blocking(net, NetworkOp::Message(message_with_marker(0x11))).await;

            let Some(NetworkResult::Message(Some(messages))) = outcome else {
                return Err(::anyhow::anyhow!("unexpected result: {outcome:?}"));
            };
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].payload[0], 0x5A);
            Ok(())
        })
        .await
        .expect("test timed out");
        result.expect("test failed");
    }

    /// A guest `sendto` forwards its bulk payload and blocks until `networkd` returns the real
    /// write response.
    #[tokio::test]
    async fn remote_sendto_round_trips_with_bulk_payload() {
        use ::syscall::sys::socket::message::SendToSocketResponse;

        let result: ::anyhow::Result<()> = timeout(TEST_TIMEOUT, async {
            let (client_end, server_end): (SocketStream, SocketStream) = unix_stream_pair();

            let (observed_tx, observed_rx) = oneshot::channel::<(ThreadIdentifier, usize, bool)>();
            let (reply_tx, reply_rx) = oneshot::channel::<()>();
            let server: JoinHandle<()> = ::tokio::spawn(async move {
                let (mut reader, mut writer): (SocketStreamReader, SocketStreamWriter) =
                    server_end.split();
                if let Some(body) = read_frame_body(&mut reader).await {
                    let request: NetworkRequest =
                        NetworkRequest::decode(&body).expect("decode request");
                    let tid: ThreadIdentifier = request_tid(&request);
                    let (len, all_ab): (usize, bool) = match &request.op {
                        NetworkOp::SendTo { data, .. } => {
                            (data.len(), data.iter().all(|&b| b == 0xAB))
                        },
                        _ => (0, false),
                    };
                    let _ = observed_tx.send((tid, len, all_ab));
                    reply_rx.await.expect("test releases sendto response");
                    writer
                        .write_all(
                            &NetworkResponse {
                                tid,
                                result: NetworkResult::SendTo(SendToSocketResponse::build(
                                    tid,
                                    i32::try_from(len).expect("payload length fits in i32"),
                                )),
                            }
                            .to_frame()
                            .expect("encode sendto response"),
                        )
                        .await
                        .expect("write sendto response");
                }
            });

            let net: Arc<RemoteNetwork> = Arc::new(RemoteNetwork::from_stream(client_end));
            let source: MessageSender =
                MessageSender::new(ProcessIdentifier::NETWORKD, ThreadIdentifier::from(7));
            let syscall_msg: SystemCallMessage = SystemCallMessage::new(
                SystemCallMessageHeader::SendToSocketRequest,
                [0u8; SystemCallMessage::PAYLOAD_SIZE],
            );
            let data: Vec<u8> = vec![0xAB; BULK_PAYLOAD_LEN];

            let client: JoinHandle<Message> = ::tokio::task::spawn_blocking(move || {
                net.handle_sendto(source, syscall_msg, &data)
            });
            let (tid, len, all_ab): (ThreadIdentifier, usize, bool) =
                observed_rx.await.expect("server received request");
            assert_eq!(tid, ThreadIdentifier::from(7));
            assert_eq!(len, BULK_PAYLOAD_LEN);
            assert!(all_ab, "bulk payload reaches networkd intact");
            assert!(!client.is_finished(), "sendto blocks until networkd replies");
            reply_tx.send(()).expect("release sendto response");
            let response: Message = client.await.expect("blocking handle_sendto");
            server.await.expect("server task");

            let status: i32 = response.status;
            assert_eq!(status, 0, "sendto reports networkd's success");
            let scm: SystemCallMessage =
                SystemCallMessage::try_from_bytes(response.payload).expect("decode reply");
            let header: SystemCallMessageHeader = scm.header;
            assert_eq!(header, SystemCallMessageHeader::SendToSocketResponse);
            let sent: i32 = SendToSocketResponse::from_bytes(scm.payload).count;
            assert_eq!(
                sent,
                i32::try_from(BULK_PAYLOAD_LEN).expect("payload length fits in i32"),
                "reports networkd's byte count"
            );
            Ok(())
        })
        .await
        .expect("test timed out");
        result.expect("test failed");
    }

    /// A `recvfrom` response carries a multi-kilobyte payload back to the client intact.
    #[tokio::test]
    async fn remote_recvfrom_round_trips_bulk_payload() {
        let result: ::anyhow::Result<()> = timeout(TEST_TIMEOUT, async {
            let (client_end, server_end): (SocketStream, SocketStream) = unix_stream_pair();
            let _server: JoinHandle<()> = spawn_responder(server_end, |request| {
                let tid: ThreadIdentifier = request_tid(&request);
                Some(NetworkResponse {
                    tid,
                    result: NetworkResult::RecvFrom {
                        msg: message_with_marker(0x33),
                        data: vec![0xCD; BULK_PAYLOAD_LEN],
                    },
                })
            });

            let net: Arc<RemoteNetwork> = Arc::new(RemoteNetwork::from_stream(client_end));
            let outcome: Option<NetworkResult> =
                round_trip_blocking(net, NetworkOp::RecvFrom(message_with_marker(0x02))).await;

            let Some(NetworkResult::RecvFrom { msg, data }) = outcome else {
                return Err(::anyhow::anyhow!("unexpected result: {outcome:?}"));
            };
            assert_eq!(msg.payload[0], 0x33);
            assert_eq!(data.len(), BULK_PAYLOAD_LEN);
            assert!(data.iter().all(|&b| b == 0xCD));
            Ok(())
        })
        .await
        .expect("test timed out");
        result.expect("test failed");
    }

    /// Two concurrent requests answered out of order are each delivered to their own caller,
    /// proving responses are correlated by guest thread identifier (`tid`) rather than arrival
    /// order.
    #[tokio::test]
    async fn remote_responses_correlate_out_of_order() {
        let result: ::anyhow::Result<()> = timeout(TEST_TIMEOUT, async {
            let (client_end, server_end): (SocketStream, SocketStream) = unix_stream_pair();

            // Read both requests first, then answer them in reverse arrival order, echoing each
            // request's marker so the client can check it received its own response.
            let server: JoinHandle<()> = ::tokio::spawn(async move {
                let (mut reader, mut writer): (SocketStreamReader, SocketStreamWriter) =
                    server_end.split();
                let first: NetworkRequest = NetworkRequest::decode(
                    &read_frame_body(&mut reader).await.expect("first frame"),
                )
                .expect("decode first");
                let second: NetworkRequest = NetworkRequest::decode(
                    &read_frame_body(&mut reader).await.expect("second frame"),
                )
                .expect("decode second");

                for request in [second, first] {
                    let NetworkOp::Message(msg) = &request.op else {
                        continue;
                    };
                    let marker: u8 = msg.payload[0];
                    let tid: ThreadIdentifier = request_tid(&request);
                    let response = NetworkResponse {
                        tid,
                        result: NetworkResult::Message(Some(vec![message_with_marker(marker)])),
                    };
                    writer
                        .write_all(&response.to_frame().expect("encode response"))
                        .await
                        .expect("write response");
                }
            });

            let net: Arc<RemoteNetwork> = Arc::new(RemoteNetwork::from_stream(client_end));
            let first: JoinHandle<Option<NetworkResult>> = {
                let net: Arc<RemoteNetwork> = net.clone();
                ::tokio::task::spawn_blocking(move || {
                    net.round_trip(NetworkOp::Message(message_with_marker(0xAA)))
                })
            };
            let second: JoinHandle<Option<NetworkResult>> = {
                let net: Arc<RemoteNetwork> = net.clone();
                ::tokio::task::spawn_blocking(move || {
                    net.round_trip(NetworkOp::Message(message_with_marker(0xBB)))
                })
            };

            let first: Option<NetworkResult> = first.await.expect("first task");
            let second: Option<NetworkResult> = second.await.expect("second task");
            server.await.expect("server task");

            let Some(NetworkResult::Message(Some(first_msgs))) = first else {
                return Err(::anyhow::anyhow!("unexpected first result: {first:?}"));
            };
            let Some(NetworkResult::Message(Some(second_msgs))) = second else {
                return Err(::anyhow::anyhow!("unexpected second result: {second:?}"));
            };
            assert_eq!(first_msgs[0].payload[0], 0xAA);
            assert_eq!(second_msgs[0].payload[0], 0xBB);
            Ok(())
        })
        .await
        .expect("test timed out");
        result.expect("test failed");
    }

    /// A `recvfrom` blocked on a lost connection is released with a `ConnectionAborted` error and no
    /// data, so the guest never deadlocks when `networkd` disappears mid-request.
    #[tokio::test]
    async fn remote_disconnect_releases_blocked_guest() {
        let result: ::anyhow::Result<()> = timeout(TEST_TIMEOUT, async {
            let (client_end, server_end): (SocketStream, SocketStream) = unix_stream_pair();

            // Read the request, then drop the connection without ever replying.
            let server: JoinHandle<()> = ::tokio::spawn(async move {
                let (mut reader, _writer): (SocketStreamReader, SocketStreamWriter) =
                    server_end.split();
                let _ = read_frame_body(&mut reader).await;
            });

            let net: Arc<RemoteNetwork> = Arc::new(RemoteNetwork::from_stream(client_end));
            let syscall_msg: SystemCallMessage = SystemCallMessage::new(
                ::syscall::SystemCallMessageHeader::ReceiveFromSocketRequest,
                [0; SystemCallMessage::PAYLOAD_SIZE],
            );
            let source: MessageSender = MessageSender::NETWORKD;

            let (msg, data): (Message, Vec<u8>) =
                ::tokio::task::spawn_blocking(move || net.handle_recvfrom(source, syscall_msg))
                    .await
                    .expect("blocking recvfrom task");
            server.await.expect("server task");

            // The guest is released with an error and no bogus payload bytes.
            assert!(data.is_empty());
            let status: i32 = msg.status;
            assert_ne!(status, 0, "expected an error status releasing the guest");
            Ok(())
        })
        .await
        .expect("test timed out");
        result.expect("test failed");
    }

    /// A guest `send` blocks until `networkd` returns the real write response.
    #[tokio::test]
    async fn remote_send_round_trips_and_blocks_until_response() {
        use ::syscall::sys::socket::message::{
            SendSocketRequest,
            SendSocketResponse,
        };

        let result: ::anyhow::Result<()> = timeout(TEST_TIMEOUT, async {
            let (client_end, server_end): (SocketStream, SocketStream) = unix_stream_pair();

            let (observed_tx, observed_rx) = oneshot::channel::<(ThreadIdentifier, i32)>();
            let (reply_tx, reply_rx) = oneshot::channel::<()>();
            let server: JoinHandle<()> = ::tokio::spawn(async move {
                let (mut reader, mut writer): (SocketStreamReader, SocketStreamWriter) =
                    server_end.split();
                if let Some(body) = read_frame_body(&mut reader).await {
                    let request: NetworkRequest =
                        NetworkRequest::decode(&body).expect("decode request");
                    let tid: ThreadIdentifier = request_tid(&request);
                    let count: i32 = match &request.op {
                        NetworkOp::Message(msg) => {
                            let scm: SystemCallMessage =
                                SystemCallMessage::try_from_bytes(msg.payload)
                                    .expect("decode send request");
                            i32::try_from(SendSocketRequest::from_bytes(scm.payload).count)
                                .expect("test request count fits in i32")
                        },
                        _ => 0,
                    };
                    let _ = observed_tx.send((tid, count));
                    reply_rx.await.expect("test releases send response");
                    writer
                        .write_all(
                            &NetworkResponse {
                                tid,
                                result: NetworkResult::Message(Some(vec![
                                    SendSocketResponse::build(tid, count),
                                ])),
                            }
                            .to_frame()
                            .expect("encode send response"),
                        )
                        .await
                        .expect("write send response");
                }
            });

            let net: Arc<RemoteNetwork> = Arc::new(RemoteNetwork::from_stream(client_end));
            let sockfd: i32 = 4096;
            let count: u32 = 5;
            let mut buffer: [u8; SendSocketRequest::BUFFER_SIZE] =
                [0; SendSocketRequest::BUFFER_SIZE];
            buffer[..5].copy_from_slice(b"hello");
            let send: Message =
                SendSocketRequest::build(ThreadIdentifier::from(7), sockfd, count, 0, buffer);

            let client: JoinHandle<Option<Vec<Message>>> =
                ::tokio::task::spawn_blocking(move || net.handle_message(send));
            let (_tid, observed_count): (ThreadIdentifier, i32) =
                observed_rx.await.expect("server received request");
            assert_eq!(observed_count, 5, "request reaches networkd with the requested count");
            assert!(!client.is_finished(), "send blocks until networkd replies");
            reply_tx.send(()).expect("release send response");
            let responses: Option<Vec<Message>> = client.await.expect("blocking handle_message");
            server.await.expect("server task");

            let responses: Vec<Message> =
                responses.ok_or_else(|| ::anyhow::anyhow!("no response"))?;
            assert_eq!(responses.len(), 1);
            let status: i32 = responses[0].status;
            assert_eq!(status, 0, "send reports networkd's success");
            let scm: SystemCallMessage =
                SystemCallMessage::try_from_bytes(responses[0].payload).expect("decode reply");
            let header: SystemCallMessageHeader = scm.header;
            assert_eq!(header, SystemCallMessageHeader::SendSocketResponse);
            let sent: i32 = SendSocketResponse::from_bytes(scm.payload).count;
            assert_eq!(sent, 5, "reports networkd's byte count");
            Ok(())
        })
        .await
        .expect("test timed out");
        result.expect("test failed");
    }

    /// Concurrent sends to the same guest socket are serialized in the client: the second request is
    /// not forwarded to `networkd` until the first write response has returned.
    #[tokio::test]
    async fn remote_sends_to_same_socket_are_serialized() {
        use ::syscall::sys::socket::message::{
            SendSocketRequest,
            SendSocketResponse,
        };

        let result: ::anyhow::Result<()> = timeout(TEST_TIMEOUT, async {
            let (client_end, server_end): (SocketStream, SocketStream) = unix_stream_pair();
            let sockfd: i32 = 4096;
            let (first_seen_tx, first_seen_rx) = oneshot::channel::<ThreadIdentifier>();
            let (early_second_tx, early_second_rx) = oneshot::channel::<bool>();
            let (release_first_tx, release_first_rx) = oneshot::channel::<()>();
            let (second_seen_tx, second_seen_rx) = oneshot::channel::<ThreadIdentifier>();
            let server: JoinHandle<()> = ::tokio::spawn(async move {
                let (mut reader, mut writer): (SocketStreamReader, SocketStreamWriter) =
                    server_end.split();
                let first: NetworkRequest = NetworkRequest::decode(
                    &read_frame_body(&mut reader).await.expect("first frame"),
                )
                .expect("decode first");
                let first_tid: ThreadIdentifier = request_tid(&first);
                let _ = first_seen_tx.send(first_tid);
                let early_second = ::tokio::time::timeout(
                    Duration::from_millis(100),
                    read_frame_body(&mut reader),
                )
                .await
                .ok()
                .flatten()
                .is_some();
                let _ = early_second_tx.send(early_second);
                release_first_rx.await.expect("test releases first send");
                writer
                    .write_all(
                        &NetworkResponse {
                            tid: first_tid,
                            result: NetworkResult::Message(Some(vec![SendSocketResponse::build(
                                first_tid, 5,
                            )])),
                        }
                        .to_frame()
                        .expect("encode first response"),
                    )
                    .await
                    .expect("write first response");
                let second: NetworkRequest = NetworkRequest::decode(
                    &read_frame_body(&mut reader).await.expect("second frame"),
                )
                .expect("decode second");
                let second_tid: ThreadIdentifier = request_tid(&second);
                let _ = second_seen_tx.send(second_tid);
                writer
                    .write_all(
                        &NetworkResponse {
                            tid: second_tid,
                            result: NetworkResult::Message(Some(vec![SendSocketResponse::build(
                                second_tid, 5,
                            )])),
                        }
                        .to_frame()
                        .expect("encode second response"),
                    )
                    .await
                    .expect("write second response");
            });

            let net: Arc<RemoteNetwork> = Arc::new(RemoteNetwork::from_stream(client_end));
            let send = |tid: i32| {
                let mut buffer: [u8; SendSocketRequest::BUFFER_SIZE] =
                    [0; SendSocketRequest::BUFFER_SIZE];
                buffer[..5].copy_from_slice(b"hello");
                SendSocketRequest::build(ThreadIdentifier::from(tid), sockfd, 5, 0, buffer)
            };
            let first = {
                let net: Arc<RemoteNetwork> = net.clone();
                let request: Message = send(11);
                ::tokio::task::spawn_blocking(move || net.handle_message(request))
            };
            assert_eq!(first_seen_rx.await.expect("first seen"), ThreadIdentifier::from(11));
            let second = {
                let net: Arc<RemoteNetwork> = net.clone();
                let request: Message = send(12);
                ::tokio::task::spawn_blocking(move || net.handle_message(request))
            };
            assert!(
                !early_second_rx.await.expect("early second result"),
                "second send must not reach networkd while first is outstanding"
            );
            release_first_tx.send(()).expect("release first send");
            let first_responses: Option<Vec<Message>> = first.await.expect("first send task");
            assert!(first_responses.is_some(), "first send completes");
            assert_eq!(
                second_seen_rx.await.expect("second seen"),
                ThreadIdentifier::from(12),
                "second send reaches networkd after first response"
            );
            let second_responses: Option<Vec<Message>> = second.await.expect("second send task");
            assert!(second_responses.is_some(), "second send completes");
            server.await.expect("server task");
            Ok(())
        })
        .await
        .expect("test timed out");
        result.expect("test failed");
    }

    /// A request issued after the reader has observed disconnect fails fast instead of registering a
    /// pending response that can never be delivered.
    #[tokio::test]
    async fn remote_request_after_disconnect_fails_fast() {
        let result: ::anyhow::Result<()> = timeout(TEST_TIMEOUT, async {
            let (client_end, server_end): (SocketStream, SocketStream) = unix_stream_pair();
            let net: Arc<RemoteNetwork> = Arc::new(RemoteNetwork::from_stream(client_end));
            drop(server_end);
            let response: Vec<Message> =
                ::tokio::task::spawn_blocking(move || net.handle_message(unparsable_message()))
                    .await
                    .expect("handle_message task")
                    .ok_or_else(|| ::anyhow::anyhow!("no response"))?;
            assert_eq!(response.len(), 1);
            let status: i32 = response[0].status;
            assert_eq!(status, i32::from(ErrorCode::ConnectionAborted));
            Ok(())
        })
        .await
        .expect("test timed out");
        result.expect("test failed");
    }

    /// The real client and the real `networkd` reactor interoperate over a socket: an inline message
    /// round-trips, and dropping the client disconnects the reactor, which returns.
    #[tokio::test]
    async fn remote_client_interoperates_with_networkd_reactor() {
        let result: ::anyhow::Result<()> = timeout(TEST_TIMEOUT, async {
            let (client_end, server_end): (SocketStream, SocketStream) = unix_stream_pair();
            let server: JoinHandle<()> = ::tokio::spawn(async move {
                let _ = ::networkd::reactor::run(HostFilter::AllowAll, server_end).await;
            });

            let net: Arc<RemoteNetwork> = Arc::new(RemoteNetwork::from_stream(client_end));
            let outcome: Option<NetworkResult> =
                round_trip_blocking(net.clone(), NetworkOp::Message(unparsable_message())).await;

            // An unparsable inline message yields no response messages, exactly as in standalone.
            let Some(NetworkResult::Message(None)) = outcome else {
                return Err(::anyhow::anyhow!("unexpected result: {outcome:?}"));
            };

            // Dropping the client closes its write half, which the reactor observes as a disconnect
            // and returns from `run` — proving the fail-stop path over a real connection.
            drop(net);
            server
                .await
                .expect("server task should return after client disconnect");
            Ok(())
        })
        .await
        .expect("test timed out");
        result.expect("test failed");
    }
}
