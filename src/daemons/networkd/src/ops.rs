// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//! Non-blocking execution of a single networking operation for the epoll reactor.
//!
//! This is the reactor-side counterpart to [`crate::dispatch`]. Both translate a decoded
//! [`NetworkOp`] into the same host-socket syscalls, but differ in readiness handling:
//!
//! [`crate::dispatch`] drives **blocking** host sockets on behalf of the in-process (standalone)
//! daemon, where `EAGAIN`/`EINPROGRESS` never arise; it is intentionally left untouched so
//! standalone semantics stay byte-for-byte identical. This module drives **non-blocking** host
//! sockets: when an operation cannot complete immediately it reports [`OpOutcome::WouldBlock`] so
//! the reactor can park it on socket readiness and retry, instead of surfacing a spurious error to
//! the guest.
//!
//! Executing an operation is idempotent with respect to parking: the reactor keeps the original
//! [`NetworkOp`] and re-invokes [`execute`] each time the socket becomes ready until the operation
//! completes. Socket writes (`send`/`sendto`) round-trip their real host outcome to the client, so a
//! short write is reported as a short successful write rather than buffered and completed later.
//==================================================================================================

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dispatch::{
        build_error,
        to_guest_fd,
        to_host_fd,
    },
    wire::{
        NetworkOp,
        NetworkResult,
    },
};
use ::log::{
    error,
    trace,
};
use ::net_backend::{
    error::NetError,
    HostFilter,
    NetBackend,
};
use ::std::os::fd::RawFd;
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageType,
    },
    pm::ThreadIdentifier,
};
use ::syscall::{
    sys::socket::{
        message::{
            AcceptSocketRequest,
            AcceptSocketResponse,
            BindSocketRequest,
            BindSocketResponse,
            ConnectSocketRequest,
            ConnectSocketResponse,
            CreateSocketPairRequest,
            CreateSocketPairResponse,
            CreateSocketRequest,
            CreateSocketResponse,
            GetPeerNameRequest,
            GetPeerNameResponse,
            GetSockNameRequest,
            GetSockNameResponse,
            ListenSocketRequest,
            ListenSocketResponse,
            ReceiveFromSocketRequest,
            ReceiveFromSocketResponse,
            ReceiveSocketRequest,
            ReceiveSocketResponse,
            SendSocketRequest,
            SendSocketResponse,
            SendToSocketRequest,
            SendToSocketResponse,
            ShutdownSocketRequest,
            ShutdownSocketResponse,
        },
        SocketAddr,
    },
    unistd::message::{
        CloseRequest,
        CloseResponse,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// The readiness direction on which a parked operation is waiting.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Waiting for the socket to become readable (`recv`, `recvfrom`, `accept`).
    Read,
    /// Waiting for the socket to become writable (`send`, `sendto`, `connect`).
    Write,
}

///
/// # Description
///
/// The result of a completed operation, together with any host-socket lifecycle side effects the
/// reactor must apply.
///
#[derive(Debug, Default)]
pub struct Completion {
    /// The response to send back to the guest, or `None` for a message the daemon drops silently
    /// (mirroring the in-process handler's behavior for unparsable/non-networking messages).
    pub response: Option<NetworkResult>,
    /// Host file descriptors newly created by this operation (already set non-blocking) that the
    /// reactor must register with its `epoll` instance and track as owned by the session.
    pub opened: Vec<RawFd>,
    /// A host file descriptor closed by this operation that the reactor must deregister and forget.
    pub closed: Option<RawFd>,
}

///
/// # Description
///
/// The outcome of a single [`execute`] attempt.
///
#[derive(Debug)]
pub enum OpOutcome {
    /// The operation completed (successfully or with an error response).
    Complete(Completion),
    /// The operation could not complete immediately; the reactor should park it on `host_fd` for
    /// readiness in `dir` and re-`execute` the same operation when the socket becomes ready.
    WouldBlock {
        /// The host socket to wait on.
        host_fd: RawFd,
        /// The readiness direction to wait for.
        dir: Direction,
    },
}

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Attempts to execute `op` against the non-blocking `backend`, enforcing `filter` on egress.
///
/// # Parameters
///
/// - `backend`: The non-blocking networking backend.
/// - `filter`: The host egress policy applied to `connect`/`sendto` destinations.
/// - `op`: The operation to execute (borrowed so it can be retried on readiness).
///
/// # Returns
///
/// [`OpOutcome::Complete`] when the operation finished, or [`OpOutcome::WouldBlock`] when it must
/// be parked on socket readiness and retried.
///
pub fn execute(backend: &NetBackend, filter: &HostFilter, op: &NetworkOp) -> OpOutcome {
    match op {
        NetworkOp::Message(msg) => execute_message(backend, filter, msg),
        NetworkOp::SendTo { msg, data } => execute_sendto(backend, filter, msg, data),
        NetworkOp::RecvFrom(msg) => execute_recvfrom(backend, msg),
    }
}

///
/// # Description
///
/// Returns the host file descriptor `op` operates on, when it references an *existing* socket.
///
/// This is the reactor's cross-session ownership hook. Before executing an operation the reactor
/// resolves the socket it names and checks that the requesting session owns it (see
/// [`reject_foreign_fd`]); without this a session could name another session's host descriptor,
/// because guest descriptors encode process-global host descriptors (see [`to_host_fd`]).
///
/// Operations that *create* a descriptor (`socket`, `socketpair`), and messages whose embedded
/// system-call payload cannot be parsed, reference no existing socket and yield [`None`].
///
pub(crate) fn op_target_fd(op: &NetworkOp) -> Option<RawFd> {
    let msg: &Message = match op {
        NetworkOp::Message(msg) | NetworkOp::RecvFrom(msg) => msg,
        NetworkOp::SendTo { msg, .. } => msg,
    };
    let syscall_msg: SystemCallMessage = SystemCallMessage::try_from_bytes(msg.payload).ok()?;
    let payload: [u8; SystemCallMessage::PAYLOAD_SIZE] = syscall_msg.payload;
    let guest_fd: i32 = match syscall_msg.header {
        SystemCallMessageHeader::CloseRequest => CloseRequest::from_bytes(payload).fd,
        SystemCallMessageHeader::BindSocketRequest => BindSocketRequest::from_bytes(payload).sockfd,
        SystemCallMessageHeader::ListenSocketRequest => {
            ListenSocketRequest::from_bytes(payload).sockfd
        },
        SystemCallMessageHeader::GetPeerNameRequest => {
            GetPeerNameRequest::from_bytes(payload).sockfd
        },
        SystemCallMessageHeader::GetSockNameRequest => {
            GetSockNameRequest::from_bytes(payload).sockfd
        },
        SystemCallMessageHeader::ShutdownSocketRequest => {
            ShutdownSocketRequest::from_bytes(payload).sockfd
        },
        SystemCallMessageHeader::ConnectSocketRequest => {
            ConnectSocketRequest::from_bytes(payload).sockfd
        },
        SystemCallMessageHeader::AcceptSocketRequest => {
            AcceptSocketRequest::from_bytes(payload).sockfd
        },
        SystemCallMessageHeader::ReceiveSocketRequest => {
            ReceiveSocketRequest::from_bytes(payload).sockfd
        },
        SystemCallMessageHeader::SendSocketRequest => SendSocketRequest::from_bytes(payload).sockfd,
        SystemCallMessageHeader::ReceiveFromSocketRequest => {
            ReceiveFromSocketRequest::from_bytes(payload).sockfd
        },
        SystemCallMessageHeader::SendToSocketRequest => {
            SendToSocketRequest::from_bytes(payload).sockfd
        },
        _ => return None,
    };
    Some(to_host_fd(guest_fd))
}

///
/// # Description
///
/// Builds the rejection outcome for an `op` that named a socket the session does not own.
///
/// A cross-session (or otherwise unknown) descriptor reference must never reach the backend, or one
/// session could operate on another's socket. The rejection mirrors the shape the operation's real
/// error would take.
///
pub(crate) fn reject_foreign_fd(
    op: &NetworkOp,
    tid: ThreadIdentifier,
    _host_fd: RawFd,
) -> OpOutcome {
    match op {
        NetworkOp::RecvFrom(_) => OpOutcome::Complete(Completion {
            response: Some(NetworkResult::RecvFrom {
                msg: build_error(tid, ErrorCode::BadFile),
                data: Vec::new(),
            }),
            ..Default::default()
        }),
        NetworkOp::SendTo { .. } => done_sendto(build_error(tid, ErrorCode::BadFile)),
        NetworkOp::Message(_) => done(build_error(tid, ErrorCode::BadFile)),
    }
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Wraps a single completed response message with no lifecycle side effects.
fn done(msg: Message) -> OpOutcome {
    OpOutcome::Complete(Completion {
        response: Some(NetworkResult::Message(Some(vec![msg]))),
        ..Default::default()
    })
}

/// Wraps a completed `sendto` response message.
fn done_sendto(msg: Message) -> OpOutcome {
    OpOutcome::Complete(Completion {
        response: Some(NetworkResult::SendTo(msg)),
        ..Default::default()
    })
}

/// Builds a completion that aborts a parked operation because its socket was closed.
pub(crate) fn abort_parked(op: NetworkOp, error: ErrorCode) -> Completion {
    let tid: ThreadIdentifier = op.tid();
    match op {
        NetworkOp::RecvFrom(_) => Completion {
            response: Some(NetworkResult::RecvFrom {
                msg: build_error(tid, error),
                data: Vec::new(),
            }),
            ..Default::default()
        },
        NetworkOp::SendTo { .. } => Completion {
            response: Some(NetworkResult::SendTo(build_error(tid, error))),
            ..Default::default()
        },
        NetworkOp::Message(_) => Completion {
            response: Some(NetworkResult::Message(Some(vec![build_error(tid, error)]))),
            ..Default::default()
        },
    }
}

/// Wraps a completed response message that also created new host sockets.
fn done_opened(msg: Message, opened: Vec<RawFd>) -> OpOutcome {
    OpOutcome::Complete(Completion {
        response: Some(NetworkResult::Message(Some(vec![msg]))),
        opened,
        ..Default::default()
    })
}

/// Dispatches an inline networking message (everything except the bulk `sendto`/`recvfrom` paths).
///
/// An inline message always produces a response, mirroring the in-process handler: an unparsable
/// payload, a message with no thread id, or a non-networking header all yield
/// [`NetworkResult::Message(None)`] rather than being dropped. Only the `sendto`/`recvfrom` paths
/// drop an unparsable message silently.
fn execute_message(backend: &NetBackend, filter: &HostFilter, msg: &Message) -> OpOutcome {
    let payload: [u8; Message::PAYLOAD_SIZE] = msg.payload;
    let syscall_msg: SystemCallMessage = match SystemCallMessage::try_from_bytes(payload) {
        Ok(syscall_msg) => syscall_msg,
        Err(_) => return message_none(),
    };
    let tid: ThreadIdentifier = msg.source.tid;
    if tid.is_none() {
        error!("networkd::reactor: inline message has no thread id");
        return message_none();
    }

    match syscall_msg.header {
        SystemCallMessageHeader::CreateSocketRequest => {
            op_socket(backend, tid, CreateSocketRequest::from_bytes(syscall_msg.payload))
        },
        SystemCallMessageHeader::CreateSocketPairRequest => {
            op_socketpair(backend, tid, CreateSocketPairRequest::from_bytes(syscall_msg.payload))
        },
        SystemCallMessageHeader::CloseRequest => {
            op_close(backend, tid, CloseRequest::from_bytes(syscall_msg.payload))
        },
        SystemCallMessageHeader::BindSocketRequest => {
            op_bind(backend, tid, BindSocketRequest::from_bytes(syscall_msg.payload))
        },
        SystemCallMessageHeader::ListenSocketRequest => {
            op_listen(backend, tid, ListenSocketRequest::from_bytes(syscall_msg.payload))
        },
        SystemCallMessageHeader::GetPeerNameRequest => {
            op_getpeername(backend, tid, GetPeerNameRequest::from_bytes(syscall_msg.payload))
        },
        SystemCallMessageHeader::GetSockNameRequest => {
            op_getsockname(backend, tid, GetSockNameRequest::from_bytes(syscall_msg.payload))
        },
        SystemCallMessageHeader::ShutdownSocketRequest => {
            op_shutdown(backend, tid, ShutdownSocketRequest::from_bytes(syscall_msg.payload))
        },
        SystemCallMessageHeader::ConnectSocketRequest => {
            op_connect(backend, filter, tid, ConnectSocketRequest::from_bytes(syscall_msg.payload))
        },
        SystemCallMessageHeader::AcceptSocketRequest => {
            op_accept(backend, tid, AcceptSocketRequest::from_bytes(syscall_msg.payload))
        },
        SystemCallMessageHeader::ReceiveSocketRequest => {
            op_recv(backend, tid, ReceiveSocketRequest::from_bytes(syscall_msg.payload))
        },
        SystemCallMessageHeader::SendSocketRequest => {
            op_send(backend, tid, SendSocketRequest::from_bytes(syscall_msg.payload))
        },
        _ => message_none(),
    }
}

/// Produces the "empty inline response" outcome ([`NetworkResult::Message(None)`]) sent for an
/// inline message the daemon accepts but does not act on (unparsable, unknown, or headerless).
fn message_none() -> OpOutcome {
    OpOutcome::Complete(Completion {
        response: Some(NetworkResult::Message(None)),
        ..Default::default()
    })
}

/// Produces the "no response" outcome for a bulk `sendto`/`recvfrom` message the daemon drops
/// silently because its embedded system-call message could not be parsed.
fn drop_message() -> OpOutcome {
    OpOutcome::Complete(Completion::default())
}

fn op_socket(
    backend: &NetBackend,
    tid: ThreadIdentifier,
    request: CreateSocketRequest,
) -> OpOutcome {
    trace!("networkd::reactor::socket(): tid={tid:?}, request={request:?}");
    match backend.socket(request.domain, request.typ, request.protocol) {
        Ok(sockfd) => match set_nonblocking(backend, sockfd) {
            Ok(()) => {
                done_opened(CreateSocketResponse::build(tid, to_guest_fd(sockfd)), vec![sockfd])
            },
            Err(code) => nonblocking_error(backend, tid, &[sockfd], code, "socket"),
        },
        Err(e) => done(net_error(tid, e)),
    }
}

fn op_socketpair(
    backend: &NetBackend,
    tid: ThreadIdentifier,
    request: CreateSocketPairRequest,
) -> OpOutcome {
    trace!("networkd::reactor::socketpair(): tid={tid:?}, request={request:?}");
    match backend.socketpair(request.domain, request.typ, request.protocol) {
        Ok((fd0, fd1)) => {
            match set_nonblocking(backend, fd0).and_then(|()| set_nonblocking(backend, fd1)) {
                Ok(()) => done_opened(
                    CreateSocketPairResponse::build(tid, to_guest_fd(fd0), to_guest_fd(fd1)),
                    vec![fd0, fd1],
                ),
                Err(code) => nonblocking_error(backend, tid, &[fd0, fd1], code, "socketpair"),
            }
        },
        Err(e) => done(net_error(tid, e)),
    }
}

fn op_close(backend: &NetBackend, tid: ThreadIdentifier, request: CloseRequest) -> OpOutcome {
    let fd: RawFd = to_host_fd(request.fd);
    trace!("networkd::reactor::close(): tid={tid:?}, fd={fd}");
    let response: Message = match backend.close(fd) {
        Ok(()) => CloseResponse::build(tid, 0, ::syscall::NETWORKD, MessageType::Ikc),
        Err(e) => net_error(tid, e),
    };
    // Regardless of the backend result, the guest fd is gone, so the reactor must stop tracking it.
    OpOutcome::Complete(Completion {
        response: Some(NetworkResult::Message(Some(vec![response]))),
        closed: Some(fd),
        ..Default::default()
    })
}

fn op_bind(backend: &NetBackend, tid: ThreadIdentifier, request: BindSocketRequest) -> OpOutcome {
    trace!("networkd::reactor::bind(): tid={tid:?}, request={request:?}");
    match backend.bind(to_host_fd(request.sockfd), &request.sockaddr) {
        Ok(()) => done(BindSocketResponse::build(tid)),
        Err(e) => done(net_error(tid, e)),
    }
}

fn op_listen(
    backend: &NetBackend,
    tid: ThreadIdentifier,
    request: ListenSocketRequest,
) -> OpOutcome {
    trace!("networkd::reactor::listen(): tid={tid:?}, request={request:?}");
    match backend.listen(to_host_fd(request.sockfd), request.backlog) {
        Ok(()) => done(ListenSocketResponse::build(tid)),
        Err(e) => done(net_error(tid, e)),
    }
}

fn op_getpeername(
    backend: &NetBackend,
    tid: ThreadIdentifier,
    request: GetPeerNameRequest,
) -> OpOutcome {
    trace!("networkd::reactor::getpeername(): tid={tid:?}, request={request:?}");
    match backend.getpeername(to_host_fd(request.sockfd)) {
        Ok(addr) => done(GetPeerNameResponse::build(tid, &addr)),
        Err(e) => done(net_error(tid, e)),
    }
}

fn op_getsockname(
    backend: &NetBackend,
    tid: ThreadIdentifier,
    request: GetSockNameRequest,
) -> OpOutcome {
    trace!("networkd::reactor::getsockname(): tid={tid:?}, request={request:?}");
    match backend.getsockname(to_host_fd(request.sockfd)) {
        Ok(addr) => done(GetSockNameResponse::build(tid, &addr)),
        Err(e) => done(net_error(tid, e)),
    }
}

fn op_shutdown(
    backend: &NetBackend,
    tid: ThreadIdentifier,
    request: ShutdownSocketRequest,
) -> OpOutcome {
    trace!("networkd::reactor::shutdown(): tid={tid:?}, request={request:?}");
    match backend.shutdown(to_host_fd(request.sockfd), request.how) {
        Ok(()) => done(ShutdownSocketResponse::build(tid)),
        Err(e) => done(net_error(tid, e)),
    }
}

fn op_connect(
    backend: &NetBackend,
    filter: &HostFilter,
    tid: ThreadIdentifier,
    request: ConnectSocketRequest,
) -> OpOutcome {
    trace!("networkd::reactor::connect(): tid={tid:?}, request={request:?}");
    let host_fd: RawFd = to_host_fd(request.sockfd);

    // Enforce host egress policy before performing the real connect. The guest never makes the
    // syscall itself, so refusing here is an unbypassable boundary. `socklen` is copied out of the
    // packed request first so nothing below takes a reference to an unaligned field.
    let socklen: usize = request.socklen as usize;
    if socklen < core::mem::size_of_val(&request.sockaddr) {
        trace!("networkd::reactor::connect(): invalid sockaddr length (socklen={socklen})");
        return done(build_error(tid, ErrorCode::InvalidArgument));
    }
    let permitted: bool = match SocketAddr::try_from(&request.sockaddr) {
        Ok(SocketAddr::V4(addr)) => filter.permits_connection(addr.addr().octets(), addr.port()),
        _ => filter.is_allow_all(),
    };
    if !permitted {
        trace!("networkd::reactor::connect(): destination denied by host egress filter");
        return done(build_error(tid, ErrorCode::PermissionDenied));
    }

    match backend.connect(host_fd, &request.sockaddr, request.socklen) {
        // Freshly established, or (on a readiness retry) already established: both are success.
        Ok(()) => done(ConnectSocketResponse::build(tid)),
        Err(NetError::Errno(ErrorCode::TransportEndpointConnected)) => {
            done(ConnectSocketResponse::build(tid))
        },
        // The handshake is still in flight; park until the socket reports writable and retry. A
        // retry issued before completion reports `EALREADY`, which is also "still in progress".
        Err(e)
            if e.is_in_progress()
                || matches!(e, NetError::Errno(ErrorCode::OperationAlreadyInProgress)) =>
        {
            OpOutcome::WouldBlock {
                host_fd,
                dir: Direction::Write,
            }
        },
        Err(e) => done(net_error(tid, e)),
    }
}

fn op_accept(
    backend: &NetBackend,
    tid: ThreadIdentifier,
    request: AcceptSocketRequest,
) -> OpOutcome {
    trace!("networkd::reactor::accept(): tid={tid:?}, request={request:?}");
    let host_fd: RawFd = to_host_fd(request.sockfd);
    match backend.accept(host_fd) {
        Ok((new_sockfd, addr)) => match set_nonblocking(backend, new_sockfd) {
            Ok(()) => done_opened(
                AcceptSocketResponse::build(tid, to_guest_fd(new_sockfd), &addr),
                vec![new_sockfd],
            ),
            Err(code) => nonblocking_error(backend, tid, &[new_sockfd], code, "accept"),
        },
        Err(e) if e.is_would_block() => OpOutcome::WouldBlock {
            host_fd,
            dir: Direction::Read,
        },
        Err(e) => done(net_error(tid, e)),
    }
}

fn op_recv(
    backend: &NetBackend,
    tid: ThreadIdentifier,
    request: ReceiveSocketRequest,
) -> OpOutcome {
    trace!("networkd::reactor::recv(): tid={tid:?}, request={request:?}");
    let host_fd: RawFd = to_host_fd(request.sockfd);
    let recv_len: usize =
        core::cmp::min(ReceiveSocketResponse::BUFFER_SIZE, request.count as usize);
    let mut buffer: [u8; ReceiveSocketResponse::BUFFER_SIZE] =
        [0; ReceiveSocketResponse::BUFFER_SIZE];
    match backend.recv(host_fd, &mut buffer, recv_len, request.flags) {
        Ok(count) => done(ReceiveSocketResponse::build(tid, count as u32, buffer)),
        Err(e) if e.is_would_block() => OpOutcome::WouldBlock {
            host_fd,
            dir: Direction::Read,
        },
        Err(e) => done(net_error(tid, e)),
    }
}

fn op_send(backend: &NetBackend, tid: ThreadIdentifier, request: SendSocketRequest) -> OpOutcome {
    trace!("networkd::reactor::send(): tid={tid:?}, request={request:?}");
    let host_fd: RawFd = to_host_fd(request.sockfd);
    let count: usize = request.count as usize;
    match backend.send(host_fd, &request.buffer, count, request.flags) {
        Ok(sent) => done(SendSocketResponse::build(tid, sent as i32)),
        Err(e) if e.is_would_block() => OpOutcome::WouldBlock {
            host_fd,
            dir: Direction::Write,
        },
        Err(e) => done(net_error(tid, e)),
    }
}

fn execute_sendto(
    backend: &NetBackend,
    filter: &HostFilter,
    msg: &Message,
    data: &[u8],
) -> OpOutcome {
    let payload: [u8; Message::PAYLOAD_SIZE] = msg.payload;
    let syscall_msg: SystemCallMessage = match SystemCallMessage::try_from_bytes(payload) {
        Ok(syscall_msg) => syscall_msg,
        Err(_) => return drop_message(),
    };
    let tid: ThreadIdentifier = msg.source.tid;
    let request: SendToSocketRequest = SendToSocketRequest::from_bytes(syscall_msg.payload);
    trace!(
        "networkd::reactor::sendto(): tid={tid:?}, request={request:?}, data.len={}",
        data.len()
    );
    let host_fd: RawFd = to_host_fd(request.sockfd);

    // Enforce host egress policy before sending to an explicit destination, mirroring `connect`.
    let permitted: bool = match SocketAddr::try_from(&request.sockaddr) {
        Ok(SocketAddr::V4(addr)) => filter.permits_connection(addr.addr().octets(), addr.port()),
        _ => filter.is_allow_all(),
    };
    if !permitted {
        trace!("networkd::reactor::sendto(): destination denied by host egress filter");
        return done_sendto(build_error(tid, ErrorCode::PermissionDenied));
    }

    match backend.sendto(host_fd, data, data.len(), request.flags, &request.sockaddr) {
        Ok(sent) => done_sendto(SendToSocketResponse::build(tid, sent as i32)),
        Err(e) if e.is_would_block() => OpOutcome::WouldBlock {
            host_fd,
            dir: Direction::Write,
        },
        Err(e) => done_sendto(net_error(tid, e)),
    }
}

fn execute_recvfrom(backend: &NetBackend, msg: &Message) -> OpOutcome {
    let payload: [u8; Message::PAYLOAD_SIZE] = msg.payload;
    let syscall_msg: SystemCallMessage = match SystemCallMessage::try_from_bytes(payload) {
        Ok(syscall_msg) => syscall_msg,
        Err(_) => return drop_message(),
    };
    let tid: ThreadIdentifier = msg.source.tid;
    let request: ReceiveFromSocketRequest =
        ReceiveFromSocketRequest::from_bytes(syscall_msg.payload);
    trace!("networkd::reactor::recvfrom(): tid={tid:?}, request={request:?}");
    let host_fd: RawFd = to_host_fd(request.sockfd);
    let recv_len: usize =
        core::cmp::min(ReceiveFromSocketResponse::MAX_DATA_SIZE, request.count as usize);
    let mut buffer: Vec<u8> = vec![0u8; recv_len];
    match backend.recvfrom(host_fd, &mut buffer, recv_len, request.flags) {
        Ok((count, addr)) => {
            let addrlen: u32 = u32::from(addr.sa_len);
            buffer.truncate(count as usize);
            OpOutcome::Complete(Completion {
                response: Some(NetworkResult::RecvFrom {
                    msg: ReceiveFromSocketResponse::build(tid, count as u32, addrlen, &addr),
                    data: buffer,
                }),
                ..Default::default()
            })
        },
        Err(e) if e.is_would_block() => OpOutcome::WouldBlock {
            host_fd,
            dir: Direction::Read,
        },
        Err(e) => OpOutcome::Complete(Completion {
            response: Some(NetworkResult::RecvFrom {
                msg: net_error(tid, e),
                data: Vec::new(),
            }),
            ..Default::default()
        }),
    }
}

/// Sets `sockfd` non-blocking, returning the mapped [`ErrorCode`] on failure.
fn set_nonblocking(backend: &NetBackend, sockfd: RawFd) -> Result<(), ErrorCode> {
    backend.set_nonblocking(sockfd, true).map_err(|e| match e {
        NetError::Interrupted => ErrorCode::Interrupted,
        NetError::Errno(code) => code,
    })
}

/// Closes sockets that were created but cannot be tracked because non-blocking setup failed.
fn nonblocking_error(
    backend: &NetBackend,
    tid: ThreadIdentifier,
    sockets: &[RawFd],
    code: ErrorCode,
    op_name: &str,
) -> OpOutcome {
    for sockfd in sockets {
        if let Err(e) = backend.close(*sockfd) {
            error!(
                "networkd::reactor::{op_name}(): failed to close fd {sockfd} after non-blocking \
                 setup failure: {e:?}"
            );
        }
    }
    done(build_error(tid, code))
}

/// Builds an error response message from a backend error.
fn net_error(tid: ThreadIdentifier, error: NetError) -> Message {
    match error {
        NetError::Interrupted => build_error(tid, ErrorCode::Interrupted),
        NetError::Errno(code) => build_error(tid, code),
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Aborting a parked inline write produces a normal correlated error response, so a client
    /// blocked waiting for `networkd` is released instead of being orphaned.
    #[test]
    fn abort_parked_inline_write_returns_error_response() {
        let tid: ThreadIdentifier = ThreadIdentifier::from(7);
        let mut buffer: [u8; SendSocketRequest::BUFFER_SIZE] = [0; SendSocketRequest::BUFFER_SIZE];
        buffer[..5].copy_from_slice(b"hello");
        let msg: Message = SendSocketRequest::build(tid, to_guest_fd(33), 5, 0, buffer);

        let completion: Completion =
            abort_parked(NetworkOp::Message(msg), ErrorCode::OperationCanceled);
        let Some(NetworkResult::Message(Some(messages))) = completion.response else {
            panic!("expected an inline error response");
        };
        assert_eq!(messages.len(), 1);
        let status: i32 = messages[0].status;
        assert_eq!(status, i32::from(ErrorCode::OperationCanceled));
    }
}
