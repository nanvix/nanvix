// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//! The epoll-driven `networkd` reactor.
//!
//! This module replaces the earlier per-request round-trip server ([`crate::server`]) with a single
//! `epoll`-driven reactor. Every host socket the daemon opens on the guest's behalf is non-blocking
//! and multiplexed through one [`Epoll`] instance, which is wrapped in a single Tokio
//! [`AsyncFd`](tokio::io::unix::AsyncFd) so the whole set can be awaited from one task.
//!
//! # Architecture
//!
//! Three cooperating tasks serve one connected user VM:
//!
//! - A **reader** task ([`read_requests`]) reads and decodes request frames and forwards them to the
//!   reactor over an [`mpsc`] channel. It is the sole owner of the read half, so
//!   [`crate::framing::read_frame`] is awaited as the only future in its task (its cancel-safety
//!   requirement).
//! - A **writer** task ([`write_responses`]) drains encoded response frames from an [`mpsc`] channel
//!   and writes them to the connection, so response bytes never interleave. Backpressure on this
//!   channel propagates all the way to the guest: a slow reader stalls the writer, which fills the
//!   channel, which parks the reactor's [`send_frames`], which stops it consuming new requests.
//! - The **reactor** task ([`event_loop`]) owns all mutable state — the backend, the `epoll`
//!   instance, and the per-session socket tables — and drives it single-threaded, so no locks are
//!   needed. It `select!`s over two cancel-safe futures: the next decoded request and `epoll`
//!   readiness. On a request it executes the operation (parking it on socket readiness if it would
//!   block); on readiness it resumes the parked operations whose sockets are now ready.
//!
//! # Multi-user-VM extensibility
//!
//! The reactor already keys its state by [`SessionId`] and routes `epoll` readiness through a
//! shared fd-ownership index, so serving several user VMs at once is a matter of accepting more than
//! one connection, spawning a reader/writer pair per connection, and inserting one [`Session`] per
//! connection. The single-VM entry point [`run`] is the degenerate case with exactly one session.
//==================================================================================================

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    epoll::{
        self,
        Epoll,
        EpollEvent,
    },
    framing,
    session::{
        self,
        Session,
        SessionId,
    },
    wire::NetworkRequest,
};
use ::log::{
    error,
    info,
    warn,
};
use ::net_backend::{
    HostFilter,
    NetBackend,
};
use ::std::{
    collections::HashMap,
    io,
    os::fd::RawFd,
};
use ::syscomm::{
    SocketStream,
    SocketStreamReader,
    SocketStreamWriter,
    WriteAll,
};
use ::tokio::{
    io::unix::AsyncFd,
    sync::mpsc,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Identifier assigned to the single user VM served by [`run`].
const SINGLE_SESSION_ID: SessionId = 0;

/// Bound on the number of decoded requests buffered between the reader task and the reactor.
const INCOMING_CHANNEL_CAPACITY: usize = 1024;

/// Bound on the number of encoded response frames buffered between the reactor and the writer task.
/// Backpressure past this point parks the reactor, propagating flow control to the guest.
const RESPONSE_CHANNEL_CAPACITY: usize = 1024;

/// Maximum number of ready events drained from `epoll` per `epoll_wait` call.
const EVENT_BATCH: usize = 64;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A decoded request delivered from a reader task to the reactor, tagged with its originating
/// session.
///
struct Incoming {
    /// The session the request belongs to.
    session: SessionId,
    /// The decoded request.
    request: NetworkRequest,
}

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Serves the single user VM connected over `stream` until it disconnects.
///
/// Creates the non-blocking backend and the `epoll` instance, spawns the reader and writer tasks
/// for the connection, and runs the reactor event loop. Returns once the user VM disconnects (or a
/// fatal reactor error occurs).
///
/// # Parameters
///
/// - `filter`: The host egress policy applied to the user VM's `connect()`/`sendto()` destinations.
/// - `stream`: The connected transport to the user VM.
///
pub async fn run(filter: HostFilter, stream: SocketStream) -> ::anyhow::Result<()> {
    let backend: NetBackend = NetBackend::new()
        .map_err(|e| ::anyhow::anyhow!("failed to initialize network backend: {e:?}"))?;
    let epoll: Epoll = Epoll::new()?;
    let async_epoll: AsyncFd<Epoll> = AsyncFd::new(epoll)?;

    let (reader, writer): (SocketStreamReader, SocketStreamWriter) = stream.split();
    let (incoming_tx, incoming_rx) = mpsc::channel::<Incoming>(INCOMING_CHANNEL_CAPACITY);
    let (response_tx, response_rx) = mpsc::channel::<Vec<u8>>(RESPONSE_CHANNEL_CAPACITY);

    // Set up the single session this daemon serves.
    let mut sessions: HashMap<SessionId, Session> = HashMap::new();
    sessions.insert(SINGLE_SESSION_ID, Session::new(SINGLE_SESSION_ID, filter, response_tx));
    let fd_owner: HashMap<RawFd, SessionId> = HashMap::new();

    let reader_task: ::tokio::task::JoinHandle<()> =
        ::tokio::spawn(read_requests(reader, SINGLE_SESSION_ID, incoming_tx));
    let writer_task: ::tokio::task::JoinHandle<()> =
        ::tokio::spawn(write_responses(writer, response_rx));

    // The event loop owns all reactor state and returns when the user VM disconnects. Dropping
    // `sessions` (which holds the response sender) closes the writer's channel, so the writer task
    // terminates once it has drained every queued frame.
    event_loop(async_epoll, backend, sessions, fd_owner, incoming_rx).await;

    // The reader may still be parked on a read if the loop exited for another reason; stop it so the
    // teardown does not hang.
    reader_task.abort();
    let _ = reader_task.await;
    if let Err(e) = writer_task.await {
        error!("networkd reactor: writer task panicked: {e}");
    }
    Ok(())
}

//==================================================================================================
// Private Functions
//==================================================================================================

///
/// # Description
///
/// Drives the reactor until every reader has disconnected or a fatal `epoll` error occurs.
///
async fn event_loop(
    async_epoll: AsyncFd<Epoll>,
    backend: NetBackend,
    mut sessions: HashMap<SessionId, Session>,
    mut fd_owner: HashMap<RawFd, SessionId>,
    mut incoming_rx: mpsc::Receiver<Incoming>,
) {
    let mut events: Vec<libc::epoll_event> = vec![epoll::empty_event(); EVENT_BATCH];
    loop {
        ::tokio::select! {
            // A newly decoded request from a reader task.
            maybe = incoming_rx.recv() => {
                let Some(incoming) = maybe else {
                    // Every reader has disconnected; the session(s) are gone.
                    info!("networkd reactor: all user VMs disconnected");
                    break;
                };
                let frames: Vec<(SessionId, Vec<u8>)> =
                    dispatch_incoming(&async_epoll, &backend, &mut sessions, &mut fd_owner, incoming);
                send_frames(&sessions, frames).await;
            }
            // The epoll set reported that one or more host sockets are ready.
            result = async_epoll.readable() => {
                let mut guard = match result {
                    Ok(guard) => guard,
                    Err(e) => {
                        error!("networkd reactor: failed to await epoll readiness: {e}");
                        break;
                    },
                };
                let frames: Vec<(SessionId, Vec<u8>)> = dispatch_epoll(
                    &async_epoll,
                    &backend,
                    &mut sessions,
                    &mut fd_owner,
                    &mut events,
                );
                // The set was fully drained (until `epoll_wait` reported nothing), so it is no longer
                // readable; clear tokio's readiness so the next edge re-wakes this task.
                guard.clear_ready();
                drop(guard);
                send_frames(&sessions, frames).await;
            }
        }
    }
}

///
/// # Description
///
/// Executes a freshly-arrived request against its session, returning any response frames produced.
///
fn dispatch_incoming(
    async_epoll: &AsyncFd<Epoll>,
    backend: &NetBackend,
    sessions: &mut HashMap<SessionId, Session>,
    fd_owner: &mut HashMap<RawFd, SessionId>,
    incoming: Incoming,
) -> Vec<(SessionId, Vec<u8>)> {
    let epoll: &Epoll = async_epoll.get_ref();
    let Incoming {
        session: sid,
        request,
    } = incoming;
    match sessions.get_mut(&sid) {
        Some(session) => session::handle_request(epoll, backend, fd_owner, session, request)
            .into_iter()
            .map(|frame| (sid, frame))
            .collect(),
        None => {
            warn!("networkd reactor: request for unknown session {sid}");
            Vec::new()
        },
    }
}

///
/// # Description
///
/// Drains every ready `epoll` event and resumes the parked operations on the ready sockets,
/// returning any response frames produced.
///
/// The ready list is drained until `epoll_wait` reports nothing so that the level-triggered set is
/// fully consumed before the caller clears tokio's readiness. This terminates because completing a
/// parked operation disarms (and, when nothing remains parked, deregisters) its socket's interest,
/// and an operation that re-blocks leaves its socket not ready, so `epoll` stops reporting it.
///
fn dispatch_epoll(
    async_epoll: &AsyncFd<Epoll>,
    backend: &NetBackend,
    sessions: &mut HashMap<SessionId, Session>,
    fd_owner: &mut HashMap<RawFd, SessionId>,
    events: &mut [libc::epoll_event],
) -> Vec<(SessionId, Vec<u8>)> {
    let epoll: &Epoll = async_epoll.get_ref();
    let mut out: Vec<(SessionId, Vec<u8>)> = Vec::new();
    loop {
        let ready: Vec<EpollEvent> = match epoll.wait(events, 0) {
            Ok(ready) => ready.iter().map(epoll::decode_event).collect(),
            // A signal interrupted the (non-blocking) wait; retry.
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                error!("networkd reactor: epoll_wait failed: {e}");
                break;
            },
        };
        if ready.is_empty() {
            break;
        }
        let drained: usize = ready.len();
        for event in ready {
            let host_fd: RawFd = session::fd_from_token(event.token);
            let Some(&sid) = fd_owner.get(&host_fd) else {
                // The fd was closed between the wait and now; ignore its stale readiness.
                continue;
            };
            if let Some(session) = sessions.get_mut(&sid) {
                let frames: Vec<Vec<u8>> = session::resume_socket(
                    epoll,
                    backend,
                    fd_owner,
                    session,
                    host_fd,
                    event.events,
                );
                out.extend(frames.into_iter().map(|frame| (sid, frame)));
            }
        }
        // A partially filled batch means the ready list is exhausted.
        if drained < events.len() {
            break;
        }
    }
    out
}

///
/// # Description
///
/// Delivers each response frame to its session's writer task, applying backpressure by awaiting the
/// bounded response channel.
///
async fn send_frames(sessions: &HashMap<SessionId, Session>, frames: Vec<(SessionId, Vec<u8>)>) {
    for (sid, frame) in frames {
        let response_tx: mpsc::Sender<Vec<u8>> = match sessions.get(&sid) {
            Some(session) => session.response_tx.clone(),
            None => continue,
        };
        if response_tx.send(frame).await.is_err() {
            warn!("networkd reactor: response channel closed before send (session {sid})");
        }
    }
}

///
/// # Description
///
/// Reads and decodes request frames from `reader`, forwarding each to the reactor over
/// `incoming_tx` until the user VM disconnects or a malformed frame desynchronizes the stream.
///
async fn read_requests(
    mut reader: SocketStreamReader,
    session: SessionId,
    incoming_tx: mpsc::Sender<Incoming>,
) {
    loop {
        let body: Vec<u8> = match framing::read_frame(&mut reader).await {
            Ok(Some(body)) => body,
            Ok(None) => {
                info!("networkd reactor: user VM disconnected");
                break;
            },
            Err(e) => {
                error!("networkd reactor: failed to read request frame: {e}");
                break;
            },
        };
        let request: NetworkRequest = match NetworkRequest::decode(&body) {
            Ok(request) => request,
            Err(e) => {
                // A malformed frame means the stream is desynchronized; fail-stop this session.
                error!("networkd reactor: malformed request frame: {e}");
                break;
            },
        };
        if incoming_tx
            .send(Incoming { session, request })
            .await
            .is_err()
        {
            // The reactor has shut down; nothing more to do.
            break;
        }
    }
}

///
/// # Description
///
/// Drains encoded response frames from `response_rx` and writes them to the connection, terminating
/// when every sender has been dropped or a write fails.
///
async fn write_responses(mut writer: SocketStreamWriter, mut response_rx: mpsc::Receiver<Vec<u8>>) {
    while let Some(frame) = response_rx.recv().await {
        if let Err(e) = writer.write_all(&frame).await {
            error!("networkd reactor: failed to write response frame: {e}");
            break;
        }
    }
}
