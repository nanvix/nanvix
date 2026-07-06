// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//! Per-connection session state and the reactor's core request/readiness handling.
//!
//! A [`Session`] represents a single connected user VM: the host egress policy applied to its
//! traffic, the channel used to return response frames to it, and the table of host sockets the
//! daemon has opened on its behalf. The daemon currently serves exactly one session, but every
//! function here is parameterized by [`Session`] so that supporting several concurrent user VMs on
//! one reactor is a matter of holding more than one [`Session`] and routing readiness to the right
//! one (see [`crate::reactor`]).
//!
//! The request/readiness handlers are free functions taking explicit references to the reactor's
//! individual pieces of state (the shared `epoll` instance, the shared backend, the fd-ownership
//! index, and the target session) rather than methods on the reactor. This keeps their borrows
//! disjoint so the single-threaded, lock-free reactor can mutate session state while holding a
//! shared reference to the `epoll` instance.
//==================================================================================================

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    epoll::{
        Epoll,
        EPOLLERR,
        EPOLLHUP,
        EPOLLIN,
        EPOLLOUT,
    },
    ops::{
        self,
        Completion,
        Direction,
        OpOutcome,
    },
    socket_state::SocketState,
    wire::{
        NetworkOp,
        NetworkRequest,
        NetworkResponse,
    },
};
use ::log::{
    error,
    warn,
};
use ::net_backend::{
    HostFilter,
    NetBackend,
};
use ::std::{
    collections::HashMap,
    os::fd::RawFd,
};
use ::sys::{
    error::ErrorCode,
    pm::ThreadIdentifier,
};
use ::tokio::sync::mpsc;

//==================================================================================================
// Structures
//==================================================================================================

/// Identifies a connected user VM served by the reactor.
pub type SessionId = u64;

///
/// # Description
///
/// The reactor's state for a single connected user VM.
///
pub struct Session {
    /// Identifies this session among all sessions the reactor serves.
    pub id: SessionId,
    /// Host egress policy applied to this session's `connect()`/`sendto()` destinations.
    pub filter: HostFilter,
    /// Channel of encoded response frames delivered to this session's writer task.
    pub response_tx: mpsc::Sender<Vec<u8>>,
    /// Host sockets opened on this session's behalf, keyed by host file descriptor.
    pub sockets: HashMap<RawFd, SocketState>,
}

impl Session {
    ///
    /// # Description
    ///
    /// Creates a new session with no open sockets.
    ///
    pub fn new(id: SessionId, filter: HostFilter, response_tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self {
            id,
            filter,
            response_tx,
            sockets: HashMap::new(),
        }
    }
}

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Handles a freshly-arrived request for `session`, executing it against the non-blocking backend.
///
/// If the operation completes immediately its response frame is returned; if it would block it is
/// parked on the owning socket and this returns no frames. New sockets created by the operation are
/// registered as owned by `session`, and a closed socket is deregistered.
///
/// # Returns
///
/// The encoded response frames produced by the request (usually zero or one).
///
pub fn handle_request(
    epoll: &Epoll,
    backend: &NetBackend,
    fd_owner: &mut HashMap<RawFd, SessionId>,
    session: &mut Session,
    request: NetworkRequest,
) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let op: NetworkOp = request.op;
    let tid: ThreadIdentifier = op.tid();
    run_op(epoll, backend, fd_owner, session, tid, op, &mut out);
    out
}

///
/// # Description
///
/// Resumes the operations parked on `host_fd` after `epoll` reported it ready with `events`.
///
/// Each parked operation in a ready direction is retried in arrival order until one still blocks or
/// the queue empties; the socket's `epoll` interest is then updated to reflect the operations that
/// remain parked (disarming and deregistering it entirely once nothing is parked).
///
/// # Returns
///
/// The encoded response frames produced by the resumed operations.
///
pub fn resume_socket(
    epoll: &Epoll,
    backend: &NetBackend,
    fd_owner: &mut HashMap<RawFd, SessionId>,
    session: &mut Session,
    host_fd: RawFd,
    events: u32,
) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();

    // An error or hangup is surfaced as readiness in both directions so any parked operation is
    // retried and observes the real error (or end-of-stream) from its syscall.
    let failed: bool = events & (EPOLLERR | EPOLLHUP) != 0;
    if failed || events & EPOLLIN != 0 {
        drain_direction(epoll, backend, fd_owner, session, host_fd, Direction::Read, &mut out);
    }
    if failed || events & EPOLLOUT != 0 {
        drain_direction(epoll, backend, fd_owner, session, host_fd, Direction::Write, &mut out);
    }

    rearm(epoll, session, host_fd);
    out
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Encodes a host file descriptor as the `epoll` registration token used to route its readiness.
pub(crate) fn token(fd: RawFd) -> u64 {
    fd as u64
}

/// Decodes an `epoll` registration token back into the host file descriptor it identifies.
pub(crate) fn fd_from_token(token: u64) -> RawFd {
    token as RawFd
}

/// Executes `op` once, completing it or parking it on its owning socket.
fn run_op(
    epoll: &Epoll,
    backend: &NetBackend,
    fd_owner: &mut HashMap<RawFd, SessionId>,
    session: &mut Session,
    tid: ThreadIdentifier,
    op: NetworkOp,
    out: &mut Vec<Vec<u8>>,
) {
    // Enforce per-session socket ownership before touching the backend: an operation may only name
    // a host socket this session opened. `session.sockets` is the session's private namespace, so a
    // descriptor it does not track is either another session's socket or one that was never opened;
    // either way it is rejected as `EBADF` and never reaches the backend. Operations that create a
    // descriptor (`socket`/`socketpair`) reference no existing socket and are always admitted.
    let outcome: OpOutcome = match ops::op_target_fd(&op) {
        Some(host_fd) if !session.sockets.contains_key(&host_fd) => {
            ops::reject_foreign_fd(&op, tid, host_fd)
        },
        _ => ops::execute(backend, &session.filter, &op),
    };
    match outcome {
        OpOutcome::Complete(completion) => {
            apply_completion(epoll, fd_owner, session, tid, completion, out);
        },
        OpOutcome::WouldBlock { host_fd, dir } => park_op(epoll, session, host_fd, dir, op),
    }
}

/// Retries the operations parked in `dir` on `host_fd`, stopping at the first that still blocks.
fn drain_direction(
    epoll: &Epoll,
    backend: &NetBackend,
    fd_owner: &mut HashMap<RawFd, SessionId>,
    session: &mut Session,
    host_fd: RawFd,
    dir: Direction,
    out: &mut Vec<Vec<u8>>,
) {
    while let Some(op) = session.sockets.get_mut(&host_fd).and_then(|s| s.pop(dir)) {
        let tid: ThreadIdentifier = op.tid();
        match ops::execute(backend, &session.filter, &op) {
            OpOutcome::Complete(completion) => {
                apply_completion(epoll, fd_owner, session, tid, completion, out);
            },
            OpOutcome::WouldBlock {
                host_fd: blocked_fd,
                dir: blocked_dir,
            } => {
                // Still not ready: restore it to the head of its queue and stop draining this
                // direction. Interest for `dir` is left armed by `rearm`.
                if let Some(sock) = session.sockets.get_mut(&blocked_fd) {
                    sock.repark_front(blocked_dir, op);
                }
                break;
            },
        }
    }
}

/// Parks `op` on `host_fd` in `dir` and arms the socket's `epoll` interest accordingly.
fn park_op(epoll: &Epoll, session: &mut Session, host_fd: RawFd, dir: Direction, op: NetworkOp) {
    let sock: &mut SocketState = match session.sockets.get_mut(&host_fd) {
        Some(sock) => sock,
        None => {
            // A socket must be tracked before an operation on it can block. A missing entry means
            // the guest referenced an fd the daemon does not own; there is nothing to park on.
            warn!("networkd reactor: parking operation on untracked fd {host_fd}");
            return;
        },
    };
    sock.park(dir, op);
    let desired: u32 = sock.desired_interest();
    update_interest(epoll, sock, desired);
}

/// Applies the lifecycle side effects of a completed operation and queues its response frame.
fn apply_completion(
    epoll: &Epoll,
    fd_owner: &mut HashMap<RawFd, SessionId>,
    session: &mut Session,
    tid: ThreadIdentifier,
    completion: Completion,
    out: &mut Vec<Vec<u8>>,
) {
    // Track sockets created by this operation. They start unregistered (interest 0) and are only
    // added to `epoll` when they first block.
    for fd in completion.opened {
        session.sockets.insert(fd, SocketState::new(fd));
        fd_owner.insert(fd, session.id);
    }

    // Stop tracking a closed socket, deregistering it from `epoll` if it was registered.
    if let Some(fd) = completion.closed {
        if let Some(mut state) = session.sockets.remove(&fd) {
            if state.interest != 0 {
                if let Err(e) = epoll.delete(fd) {
                    error!("networkd reactor: failed to deregister fd {fd} from epoll: {e}");
                }
            }
            let parked_reads: Vec<NetworkOp> = state.parked_read.drain(..).collect();
            for op in parked_reads.into_iter().chain(state.parked_write.drain(..)) {
                let tid: ThreadIdentifier = op.tid();
                if let Some(result) = ops::abort_parked(op, ErrorCode::OperationCanceled).response {
                    out.push(NetworkResponse { tid, result }.to_frame());
                }
            }
        }
        fd_owner.remove(&fd);
    }

    if let Some(result) = completion.response {
        out.push(NetworkResponse { tid, result }.to_frame());
    }
}

/// Recomputes and applies `host_fd`'s `epoll` interest after its parked operations changed.
fn rearm(epoll: &Epoll, session: &mut Session, host_fd: RawFd) {
    if let Some(sock) = session.sockets.get_mut(&host_fd) {
        let desired: u32 = sock.desired_interest();
        update_interest(epoll, sock, desired);
    }
}

/// Transitions `sock` to the `desired` `epoll` interest, registering, modifying, or deregistering
/// it as required. A socket is registered with `epoll` exactly while its interest is non-zero.
fn update_interest(epoll: &Epoll, sock: &mut SocketState, desired: u32) {
    if desired == sock.interest {
        return;
    }
    let host_fd: RawFd = sock.host_fd;
    let result: ::std::io::Result<()> = if sock.interest == 0 {
        epoll.add(host_fd, token(host_fd), desired)
    } else if desired == 0 {
        epoll.delete(host_fd)
    } else {
        epoll.modify(host_fd, token(host_fd), desired)
    };
    if let Err(e) = result {
        error!("networkd reactor: failed to update epoll interest for fd {host_fd}: {e}");
    }
    sock.interest = desired;
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dispatch::to_guest_fd,
        epoll::{
            decode_event,
            empty_event,
        },
        wire::NetworkResult,
    };
    use ::sys::{
        error::ErrorCode,
        ipc::Message,
    };
    use ::sysapi::sys_socket::{
        sockaddr,
        socklen_t,
    };
    use ::syscall::{
        netinet::in_::{
            Ipv4Addr,
            Protocol,
            SocketAddrV4,
        },
        sys::socket::{
            message::{
                ConnectSocketRequest,
                ReceiveSocketRequest,
                ReceiveSocketResponse,
                SendSocketRequest,
                SendSocketResponse,
            },
            AddressFamily,
            SocketAddr,
            SocketType,
        },
        SystemCallMessage,
        SystemCallMessageHeader,
    };

    /// Decodes the response carried by a length-prefixed wire frame produced by the reactor.
    fn decode_frame(frame: &[u8]) -> NetworkResponse {
        // The frame is a little-endian `u32` body length followed by the encoded body.
        NetworkResponse::decode(&frame[4..]).expect("frame should decode")
    }

    /// Extracts the `(count, data)` of the recv reply carried by an inline message response.
    fn recv_reply(response: &NetworkResponse) -> (usize, Vec<u8>) {
        let messages: &Vec<Message> = match &response.result {
            NetworkResult::Message(Some(messages)) => messages,
            other => panic!("expected inline message response, got {other:?}"),
        };
        assert_eq!(messages.len(), 1, "recv reply carries exactly one message");
        let payload: [u8; Message::PAYLOAD_SIZE] = messages[0].payload;
        let syscall_msg: SystemCallMessage =
            SystemCallMessage::try_from_bytes(payload).expect("recv reply payload decodes");
        let reply: ReceiveSocketResponse = ReceiveSocketResponse::from_bytes(syscall_msg.payload);
        let count: usize = reply.count as usize;
        (count, reply.buffer[..count].to_vec())
    }

    /// A recv that finds no data parks on readability and is resumed once a peer send delivers it,
    /// after which the socket's `epoll` interest is disarmed.
    #[test]
    fn recv_parks_then_resumes_after_peer_send() {
        let backend: NetBackend = NetBackend::new().expect("backend init");
        let epoll: Epoll = Epoll::new().expect("epoll_create1");

        // A connected pair of non-blocking host sockets standing in for a guest socket and its peer.
        let (reader, writer): (RawFd, RawFd) = backend
            .socketpair(AddressFamily::Unix, SocketType::Stream, Protocol::Ip)
            .expect("socketpair");
        backend
            .set_nonblocking(reader, true)
            .expect("nonblocking reader");
        backend
            .set_nonblocking(writer, true)
            .expect("nonblocking writer");

        let (response_tx, _response_rx) = mpsc::channel::<Vec<u8>>(16);
        let mut session: Session = Session::new(0, HostFilter::AllowAll, response_tx);
        session.sockets.insert(reader, SocketState::new(reader));
        session.sockets.insert(writer, SocketState::new(writer));
        let mut fd_owner: HashMap<RawFd, SessionId> = HashMap::new();
        fd_owner.insert(reader, session.id);
        fd_owner.insert(writer, session.id);

        // A recv on the empty reader would block, so it is parked and produces no response yet.
        let recv_tid: ThreadIdentifier = ThreadIdentifier::from(7);
        let recv: NetworkRequest = NetworkRequest {
            op: NetworkOp::Message(ReceiveSocketRequest::build(
                recv_tid,
                to_guest_fd(reader),
                64,
                0,
            )),
        };
        let frames: Vec<Vec<u8>> =
            handle_request(&epoll, &backend, &mut fd_owner, &mut session, recv);
        assert!(frames.is_empty(), "a blocked recv yields no response");
        let state: &SocketState = session.sockets.get(&reader).expect("reader tracked");
        assert_eq!(state.interest, EPOLLIN, "reader is armed for readability");
        assert_eq!(state.parked_read.len(), 1, "recv is parked");

        // A send on the peer responds with the real host write result and makes the reader readable.
        let mut buffer: [u8; SendSocketRequest::BUFFER_SIZE] = [0; SendSocketRequest::BUFFER_SIZE];
        buffer[..5].copy_from_slice(b"hello");
        let send_tid: ThreadIdentifier = ThreadIdentifier::from(9);
        let send: NetworkRequest = NetworkRequest {
            op: NetworkOp::Message(SendSocketRequest::build(
                send_tid,
                to_guest_fd(writer),
                5,
                0,
                buffer,
            )),
        };
        let frames: Vec<Vec<u8>> =
            handle_request(&epoll, &backend, &mut fd_owner, &mut session, send);
        assert_eq!(frames.len(), 1, "a completed send yields a response");
        let response: NetworkResponse = decode_frame(&frames[0]);
        assert_eq!(response.tid, send_tid, "send response correlates with the sender");
        let messages: &Vec<Message> = match &response.result {
            NetworkResult::Message(Some(messages)) => messages,
            other => panic!("expected an inline send response, got {other:?}"),
        };
        let payload: [u8; Message::PAYLOAD_SIZE] = messages[0].payload;
        let syscall_msg: SystemCallMessage =
            SystemCallMessage::try_from_bytes(payload).expect("send reply decodes");
        let sent: i32 = SendSocketResponse::from_bytes(syscall_msg.payload).count;
        assert_eq!(sent, 5, "send reports the real written byte count");

        // The reactor's epoll now reports the reader ready; the parked recv is resumed.
        let mut events: [libc::epoll_event; 8] = [empty_event(); 8];
        let ready = epoll.wait(&mut events, 1_000).expect("epoll wait");
        assert_eq!(ready.len(), 1, "exactly the reader is ready");
        let event = decode_event(&ready[0]);
        assert_eq!(fd_from_token(event.token), reader, "readiness routes to the reader");
        assert_ne!(event.events & EPOLLIN, 0, "reader reports readability");

        let frames: Vec<Vec<u8>> =
            resume_socket(&epoll, &backend, &mut fd_owner, &mut session, reader, event.events);
        assert_eq!(frames.len(), 1, "the resumed recv produces its response");

        let response: NetworkResponse = decode_frame(&frames[0]);
        assert_eq!(response.tid, recv_tid, "response correlates with the recv thread id");
        let (count, data) = recv_reply(&response);
        assert_eq!(count, 5, "recv returns the five sent bytes");
        assert_eq!(&data, b"hello", "recv returns the sent payload");

        // With nothing left parked the reader is disarmed and deregistered from epoll.
        let state: &SocketState = session.sockets.get(&reader).expect("reader still tracked");
        assert!(state.parked_read.is_empty(), "no recv remains parked");
        assert_eq!(state.interest, 0, "reader is disarmed once drained");
        let mut events: [libc::epoll_event; 8] = [empty_event(); 8];
        let ready = epoll.wait(&mut events, 0).expect("epoll wait");
        assert!(ready.is_empty(), "a disarmed socket reports no readiness");
    }

    /// A session may only operate on the sockets it opened: an operation naming another session's
    /// host descriptor is rejected as `EBADF` and never reaches the backend, leaving the owner's
    /// socket untouched. This proves the isolation the reactor relies on to serve several user VMs.
    #[test]
    fn foreign_socket_reference_is_rejected() {
        let backend: NetBackend = NetBackend::new().expect("backend init");
        let epoll: Epoll = Epoll::new().expect("epoll_create1");

        // Session A owns a connected pair of non-blocking host sockets.
        let (a_reader, a_writer): (RawFd, RawFd) = backend
            .socketpair(AddressFamily::Unix, SocketType::Stream, Protocol::Ip)
            .expect("socketpair");
        backend
            .set_nonblocking(a_reader, true)
            .expect("nonblocking reader");
        backend
            .set_nonblocking(a_writer, true)
            .expect("nonblocking writer");

        let (a_tx, _a_rx) = mpsc::channel::<Vec<u8>>(16);
        let mut session_a: Session = Session::new(0, HostFilter::AllowAll, a_tx);
        session_a
            .sockets
            .insert(a_reader, SocketState::new(a_reader));
        session_a
            .sockets
            .insert(a_writer, SocketState::new(a_writer));

        let mut fd_owner: HashMap<RawFd, SessionId> = HashMap::new();
        fd_owner.insert(a_reader, session_a.id);
        fd_owner.insert(a_writer, session_a.id);

        // Session B owns nothing and tries to recv on session A's reader.
        let (b_tx, _b_rx) = mpsc::channel::<Vec<u8>>(16);
        let mut session_b: Session = Session::new(1, HostFilter::AllowAll, b_tx);

        let tid: ThreadIdentifier = ThreadIdentifier::from(11);
        let recv: NetworkRequest = NetworkRequest {
            op: NetworkOp::Message(ReceiveSocketRequest::build(tid, to_guest_fd(a_reader), 64, 0)),
        };
        let frames: Vec<Vec<u8>> =
            handle_request(&epoll, &backend, &mut fd_owner, &mut session_b, recv);

        // B receives an `EBADF` error response for the foreign descriptor.
        assert_eq!(frames.len(), 1, "the rejected recv produces exactly one response");
        let response: NetworkResponse = decode_frame(&frames[0]);
        assert_eq!(response.tid, tid, "response correlates with the requesting thread id");
        let messages: &Vec<Message> = match &response.result {
            NetworkResult::Message(Some(messages)) => messages,
            other => panic!("expected an inline error response, got {other:?}"),
        };
        assert_eq!(messages.len(), 1, "the error response carries one message");
        let status: i32 = messages[0].status;
        assert_eq!(
            status,
            i32::from(ErrorCode::BadFile),
            "the foreign reference is rejected as EBADF"
        );

        // Session A's socket is untouched: B tracks nothing, nothing was parked on A's reader, and
        // A retains ownership.
        assert!(session_b.sockets.is_empty(), "the rejected op tracks no socket for B");
        let a_state: &SocketState = session_a
            .sockets
            .get(&a_reader)
            .expect("A still owns its reader");
        assert!(a_state.parked_read.is_empty(), "no operation was parked on A's reader");
        assert_eq!(
            fd_owner.get(&a_reader),
            Some(&session_a.id),
            "A retains ownership of its reader"
        );
    }

    /// A send that finds the host socket full parks and later resumes with a normal response. The
    /// client is blocked while this request is parked, so the reactor does not need to buffer a
    /// queue of additional writes for the socket.
    #[test]
    fn send_parks_then_resumes_with_response() {
        let backend: NetBackend = NetBackend::new().expect("backend init");
        let epoll: Epoll = Epoll::new().expect("epoll_create1");

        // A connected pair of non-blocking host sockets standing in for a guest socket and its
        // peer. The writer's send buffer is shrunk to the kernel minimum and then filled so the
        // guest send below parks on writability.
        let (writer, reader): (RawFd, RawFd) = backend
            .socketpair(AddressFamily::Unix, SocketType::Stream, Protocol::Ip)
            .expect("socketpair");
        backend
            .set_nonblocking(writer, true)
            .expect("nonblocking writer");
        backend
            .set_nonblocking(reader, true)
            .expect("nonblocking reader");
        let sndbuf: libc::c_int = 1;
        let rc: libc::c_int = unsafe {
            libc::setsockopt(
                writer,
                libc::SOL_SOCKET,
                libc::SO_SNDBUF,
                (&raw const sndbuf).cast(),
                core::mem::size_of::<libc::c_int>() as libc::socklen_t,
            )
        };
        assert_eq!(rc, 0, "shrinking the writer's send buffer succeeds");
        let filler: [u8; SendSocketRequest::BUFFER_SIZE] = [0xEE; SendSocketRequest::BUFFER_SIZE];
        for _ in 0..1024 {
            match backend.send(writer, &filler, filler.len(), 0) {
                Ok(0) => break,
                Ok(_) => {},
                Err(e) if e.is_would_block() => break,
                Err(e) => panic!("filling writer failed: {e:?}"),
            }
        }

        let (response_tx, _response_rx) = mpsc::channel::<Vec<u8>>(16);
        let mut session: Session = Session::new(0, HostFilter::AllowAll, response_tx);
        session.sockets.insert(writer, SocketState::new(writer));
        session.sockets.insert(reader, SocketState::new(reader));
        let mut fd_owner: HashMap<RawFd, SessionId> = HashMap::new();
        fd_owner.insert(writer, session.id);
        fd_owner.insert(reader, session.id);

        let send_tid: ThreadIdentifier = ThreadIdentifier::from(42);
        let mut buffer: [u8; SendSocketRequest::BUFFER_SIZE] = [0; SendSocketRequest::BUFFER_SIZE];
        buffer[..5].copy_from_slice(b"hello");
        let send: NetworkRequest = NetworkRequest {
            op: NetworkOp::Message(SendSocketRequest::build(
                send_tid,
                to_guest_fd(writer),
                5,
                0,
                buffer,
            )),
        };
        let frames: Vec<Vec<u8>> =
            handle_request(&epoll, &backend, &mut fd_owner, &mut session, send);
        assert!(frames.is_empty(), "a blocked send yields no response until writable");
        let state: &SocketState = session.sockets.get(&writer).expect("writer tracked");
        assert_eq!(state.parked_write.len(), 1, "the send is parked");

        let mut buf: [u8; 4096] = [0; 4096];
        loop {
            let len: usize = buf.len();
            match backend.recv(reader, &mut buf, len, 0) {
                Ok(0) => break,
                Ok(_) => {},
                Err(e) if e.is_would_block() => break,
                Err(e) => panic!("peer recv failed: {e:?}"),
            }
        }
        let mut events: [libc::epoll_event; 8] = [empty_event(); 8];
        let ready = epoll.wait(&mut events, 1_000).expect("epoll wait");
        assert!(!ready.is_empty(), "writer becomes writable after peer drains");
        let event = decode_event(&ready[0]);
        assert_eq!(fd_from_token(event.token), writer, "readiness routes to writer");
        let frames: Vec<Vec<u8>> =
            resume_socket(&epoll, &backend, &mut fd_owner, &mut session, writer, event.events);
        assert_eq!(frames.len(), 1, "the resumed send produces its response");
        let response: NetworkResponse = decode_frame(&frames[0]);
        assert_eq!(response.tid, send_tid, "send response preserves tid");
        let messages: &Vec<Message> = match &response.result {
            NetworkResult::Message(Some(messages)) => messages,
            other => panic!("expected an inline send response, got {other:?}"),
        };
        let syscall_msg: SystemCallMessage =
            SystemCallMessage::try_from_bytes(messages[0].payload).expect("send reply decodes");
        let sent: i32 = SendSocketResponse::from_bytes(syscall_msg.payload).count;
        assert!(sent > 0 && sent <= 5, "send reports actual progress (sent={sent})");
        let state: &SocketState = session.sockets.get(&writer).expect("writer tracked");
        assert!(state.parked_write.is_empty(), "the send is no longer parked");
    }

    /// A non-blocking `connect()` to a live loopback listener returns `EINPROGRESS`, so the reactor
    /// parks it on writability and yields no response yet; once the handshake completes, `epoll`
    /// reports the socket writable and the resumed connect completes successfully. This is exactly
    /// the path a guest `connect()` takes through the decoupled reactor, and the intermediate
    /// `EINPROGRESS` must never be surfaced to the guest as an error.
    #[test]
    fn connect_parks_then_resumes_when_writable() {
        // A real loopback listener stands in for the peer the guest connects to.
        let listener: ::std::net::TcpListener =
            ::std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
        let port: u16 = listener.local_addr().expect("listener addr").port();

        let backend: NetBackend = NetBackend::new().expect("backend init");
        let epoll: Epoll = Epoll::new().expect("epoll_create1");

        // A non-blocking client socket, tracked by the session exactly as `op_socket` would leave
        // it after creating it.
        let client: RawFd = backend
            .socket(AddressFamily::Inet, SocketType::Stream, Protocol::Tcp)
            .expect("client socket");
        backend
            .set_nonblocking(client, true)
            .expect("nonblocking client");

        let (response_tx, _response_rx) = mpsc::channel::<Vec<u8>>(16);
        let mut session: Session = Session::new(0, HostFilter::AllowAll, response_tx);
        session.sockets.insert(client, SocketState::new(client));
        let mut fd_owner: HashMap<RawFd, SessionId> = HashMap::new();
        fd_owner.insert(client, session.id);

        // Build a connect request aimed at the loopback listener.
        let dst: SocketAddr =
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new([127, 0, 0, 1]), port));
        let (addr, socklen): (sockaddr, socklen_t) = <(sockaddr, socklen_t)>::from(dst);
        let connect_tid: ThreadIdentifier = ThreadIdentifier::from(11);
        let connect: NetworkRequest = NetworkRequest {
            op: NetworkOp::Message(ConnectSocketRequest::build(
                connect_tid,
                to_guest_fd(client),
                &addr,
                socklen,
            )),
        };

        // The connect either completes synchronously (fast loopback) or parks on writability. In
        // both cases the guest must ultimately observe a success, never the intermediate
        // `EINPROGRESS`.
        let mut frames: Vec<Vec<u8>> =
            handle_request(&epoll, &backend, &mut fd_owner, &mut session, connect);
        if frames.is_empty() {
            let state: &SocketState = session.sockets.get(&client).expect("client tracked");
            assert_eq!(state.interest, EPOLLOUT, "client is armed for writability");
            assert_eq!(state.parked_write.len(), 1, "connect is parked on writability");

            // The loopback handshake completes; epoll reports the client writable.
            let mut events: [libc::epoll_event; 8] = [empty_event(); 8];
            let ready = epoll.wait(&mut events, 1_000).expect("epoll wait");
            assert_eq!(ready.len(), 1, "exactly the client is ready");
            let event = decode_event(&ready[0]);
            assert_eq!(fd_from_token(event.token), client, "readiness routes to the client");
            assert_ne!(event.events & EPOLLOUT, 0, "client reports writability");

            frames =
                resume_socket(&epoll, &backend, &mut fd_owner, &mut session, client, event.events);
        }

        // Exactly one response is produced.
        assert_eq!(frames.len(), 1, "the connect produces exactly one response");
        let response: NetworkResponse = decode_frame(&frames[0]);
        assert_eq!(response.tid, connect_tid, "response correlates with the connect thread id");
        let messages: &Vec<Message> = match &response.result {
            NetworkResult::Message(Some(messages)) => messages,
            other => panic!("expected an inline connect response, got {other:?}"),
        };
        assert_eq!(messages.len(), 1, "the connect reply carries one message");
        let status: i32 = messages[0].status;
        let payload: [u8; Message::PAYLOAD_SIZE] = messages[0].payload;
        let syscall_msg: SystemCallMessage =
            SystemCallMessage::try_from_bytes(payload).expect("connect reply decodes");
        let header: SystemCallMessageHeader = syscall_msg.header;
        assert_eq!(status, 0, "the connect reports success, not an error (status={status})");
        assert_eq!(
            header,
            SystemCallMessageHeader::ConnectSocketResponse,
            "the resumed connect reports success, not an error"
        );

        // With nothing left parked the client is disarmed and deregistered from epoll.
        let state: &SocketState = session.sockets.get(&client).expect("client still tracked");
        assert!(state.parked_write.is_empty(), "no connect remains parked");
        assert_eq!(state.interest, 0, "client is disarmed once connected");
    }
}
