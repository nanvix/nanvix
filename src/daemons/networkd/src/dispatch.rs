// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::log::{
    error,
    trace,
};
use ::net_backend::{
    error::NetError,
    HostFilter,
    NetBackend,
};
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
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
            ReceiveSocketRequest,
            ReceiveSocketResponse,
            SendSocketRequest,
            SendSocketResponse,
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
    SOCKET_FD_BASE,
};

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Determines whether a given message header corresponds to a networking system call.
///
pub fn is_networking_header(header: &SystemCallMessageHeader) -> bool {
    matches!(
        header,
        SystemCallMessageHeader::AcceptSocketRequest
            | SystemCallMessageHeader::BindSocketRequest
            | SystemCallMessageHeader::CloseRequest
            | SystemCallMessageHeader::ConnectSocketRequest
            | SystemCallMessageHeader::CreateSocketPairRequest
            | SystemCallMessageHeader::CreateSocketRequest
            | SystemCallMessageHeader::GetPeerNameRequest
            | SystemCallMessageHeader::GetSockNameRequest
            | SystemCallMessageHeader::ListenSocketRequest
            | SystemCallMessageHeader::ReceiveSocketRequest
            | SystemCallMessageHeader::SendSocketRequest
            | SystemCallMessageHeader::ShutdownSocketRequest
    )
}

///
/// # Description
///
/// Dispatches a networking system call message to the appropriate handler.
///
/// # Parameters
///
/// - `backend`: Reference to the networking backend.
/// - `source`: The message source (identifies the calling thread).
/// - `syscall_msg`: The parsed system call message.
///
/// # Returns
///
/// A vector of response messages on success, or `None` if the message is not a networking
/// message.
///
pub fn dispatch_message(
    backend: &NetBackend,
    filter: &HostFilter,
    source: MessageSender,
    syscall_msg: SystemCallMessage,
) -> Option<Vec<Message>> {
    let tid: ThreadIdentifier = match source.as_id() {
        Err(tid) => tid,
        Ok(pid) => {
            error!("networkd::dispatch(): message source is a PID ({pid:?}), expected TID");
            return None;
        },
    };

    match syscall_msg.header {
        SystemCallMessageHeader::AcceptSocketRequest => {
            let request: AcceptSocketRequest = AcceptSocketRequest::from_bytes(syscall_msg.payload);
            Some(vec![do_accept(backend, tid, request)])
        },
        SystemCallMessageHeader::BindSocketRequest => {
            let request: BindSocketRequest = BindSocketRequest::from_bytes(syscall_msg.payload);
            Some(vec![do_bind(backend, tid, request)])
        },
        SystemCallMessageHeader::CloseRequest => {
            let request: CloseRequest = CloseRequest::from_bytes(syscall_msg.payload);
            Some(vec![do_close(backend, tid, request)])
        },
        SystemCallMessageHeader::ConnectSocketRequest => {
            let request: ConnectSocketRequest =
                ConnectSocketRequest::from_bytes(syscall_msg.payload);
            Some(vec![do_connect(backend, filter, tid, request)])
        },
        SystemCallMessageHeader::CreateSocketPairRequest => {
            let request: CreateSocketPairRequest =
                CreateSocketPairRequest::from_bytes(syscall_msg.payload);
            Some(vec![do_socketpair(backend, tid, request)])
        },
        SystemCallMessageHeader::CreateSocketRequest => {
            let request: CreateSocketRequest = CreateSocketRequest::from_bytes(syscall_msg.payload);
            Some(vec![do_socket(backend, tid, request)])
        },
        SystemCallMessageHeader::GetPeerNameRequest => {
            let request: GetPeerNameRequest = GetPeerNameRequest::from_bytes(syscall_msg.payload);
            Some(vec![do_getpeername(backend, tid, request)])
        },
        SystemCallMessageHeader::GetSockNameRequest => {
            let request: GetSockNameRequest = GetSockNameRequest::from_bytes(syscall_msg.payload);
            Some(vec![do_getsockname(backend, tid, request)])
        },
        SystemCallMessageHeader::ListenSocketRequest => {
            let request: ListenSocketRequest = ListenSocketRequest::from_bytes(syscall_msg.payload);
            Some(vec![do_listen(backend, tid, request)])
        },
        SystemCallMessageHeader::ReceiveSocketRequest => {
            let request: ReceiveSocketRequest =
                ReceiveSocketRequest::from_bytes(syscall_msg.payload);
            Some(vec![do_recv(backend, tid, request)])
        },
        SystemCallMessageHeader::SendSocketRequest => {
            let request: SendSocketRequest = SendSocketRequest::from_bytes(syscall_msg.payload);
            Some(vec![do_send(backend, tid, request)])
        },
        SystemCallMessageHeader::ShutdownSocketRequest => {
            let request: ShutdownSocketRequest =
                ShutdownSocketRequest::from_bytes(syscall_msg.payload);
            Some(vec![do_shutdown(backend, tid, request)])
        },
        _ => None,
    }
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Shifts a host-side file descriptor into the guest socket fd range.
fn to_guest_fd(host_fd: i32) -> i32 {
    host_fd + SOCKET_FD_BASE
}

/// Shifts a guest socket file descriptor back to the host-side value.
fn to_host_fd(guest_fd: i32) -> i32 {
    guest_fd - SOCKET_FD_BASE
}

fn build_error(tid: ThreadIdentifier, error: ErrorCode) -> Message {
    Message::new(
        MessageSender::from(::syscall::NETWORKD),
        MessageReceiver::from(tid),
        MessageType::Ikc,
        Some(error),
        [0u8; Message::PAYLOAD_SIZE],
    )
}

fn do_socket(backend: &NetBackend, tid: ThreadIdentifier, request: CreateSocketRequest) -> Message {
    trace!("networkd::socket(): tid={tid:?}, request={request:?}");
    match backend.socket(request.domain, request.typ, request.protocol) {
        Ok(sockfd) => CreateSocketResponse::build(tid, to_guest_fd(sockfd)),
        Err(NetError::Interrupted) => build_error(tid, ErrorCode::Interrupted),
        Err(NetError::Errno(code)) => build_error(tid, code),
    }
}

fn do_close(backend: &NetBackend, tid: ThreadIdentifier, request: CloseRequest) -> Message {
    let fd: i32 = to_host_fd(request.fd);
    trace!("networkd::close(): tid={tid:?}, fd={fd}");
    match backend.close(fd) {
        Ok(()) => CloseResponse::build(tid, 0, ::syscall::NETWORKD, MessageType::Ikc),
        Err(NetError::Interrupted) => build_error(tid, ErrorCode::Interrupted),
        Err(NetError::Errno(code)) => build_error(tid, code),
    }
}

fn do_socketpair(
    backend: &NetBackend,
    tid: ThreadIdentifier,
    request: CreateSocketPairRequest,
) -> Message {
    trace!("networkd::socketpair(): tid={tid:?}, request={request:?}");
    match backend.socketpair(request.domain, request.typ, request.protocol) {
        Ok((fd0, fd1)) => CreateSocketPairResponse::build(tid, to_guest_fd(fd0), to_guest_fd(fd1)),
        Err(NetError::Interrupted) => build_error(tid, ErrorCode::Interrupted),
        Err(NetError::Errno(code)) => build_error(tid, code),
    }
}

fn do_bind(backend: &NetBackend, tid: ThreadIdentifier, request: BindSocketRequest) -> Message {
    trace!("networkd::bind(): tid={tid:?}, request={request:?}");
    match backend.bind(to_host_fd(request.sockfd), &request.sockaddr) {
        Ok(()) => BindSocketResponse::build(tid),
        Err(NetError::Interrupted) => build_error(tid, ErrorCode::Interrupted),
        Err(NetError::Errno(code)) => build_error(tid, code),
    }
}

fn do_connect(
    backend: &NetBackend,
    filter: &HostFilter,
    tid: ThreadIdentifier,
    request: ConnectSocketRequest,
) -> Message {
    trace!("networkd::connect(): tid={tid:?}, request={request:?}");
    // Enforce host egress policy before performing the real connect. The guest
    // never makes the syscall itself, so refusing here is an unbypassable
    // boundary.
    //
    // IPv4 destinations are matched against the filter. Any non-IPv4 destination
    // (e.g. AF_UNIX, which would otherwise let a guest reach host-local sockets)
    // or an unparsable address cannot be evaluated by the IPv4 filter, so it is
    // denied whenever a filter is active and permitted only under `AllowAll`
    // (preserving unrestricted behavior when no policy is set).
    //
    // Connections to the DNS port are exempted in allowlist mode so name
    // resolution works for the allowed hosts (see `HostFilter::permits_connection`).
    let min_socklen: usize = core::mem::size_of::<::syscall::sys::socket::sockaddr>();
    if (request.socklen as usize) < min_socklen {
        trace!("networkd::connect(): invalid sockaddr length (socklen={})", request.socklen);
        return build_error(tid, ErrorCode::InvalidArgument);
    }
    let permitted: bool = match SocketAddr::try_from(&request.sockaddr) {
        Ok(SocketAddr::V4(addr)) => filter.permits_connection(addr.addr().octets(), addr.port()),
        _ => filter.is_allow_all(),
    };
    if !permitted {
        trace!("networkd::connect(): destination denied by host egress filter");
        return build_error(tid, ErrorCode::PermissionDenied);
    }
    match backend.connect(to_host_fd(request.sockfd), &request.sockaddr, request.socklen) {
        Ok(()) => ConnectSocketResponse::build(tid),
        Err(NetError::Interrupted) => build_error(tid, ErrorCode::Interrupted),
        Err(NetError::Errno(code)) => build_error(tid, code),
    }
}

fn do_listen(backend: &NetBackend, tid: ThreadIdentifier, request: ListenSocketRequest) -> Message {
    trace!("networkd::listen(): tid={tid:?}, request={request:?}");
    match backend.listen(to_host_fd(request.sockfd), request.backlog) {
        Ok(()) => ListenSocketResponse::build(tid),
        Err(NetError::Interrupted) => build_error(tid, ErrorCode::Interrupted),
        Err(NetError::Errno(code)) => build_error(tid, code),
    }
}

fn do_getpeername(
    backend: &NetBackend,
    tid: ThreadIdentifier,
    request: GetPeerNameRequest,
) -> Message {
    trace!("networkd::getpeername(): tid={tid:?}, request={request:?}");
    match backend.getpeername(to_host_fd(request.sockfd)) {
        Ok(addr) => GetPeerNameResponse::build(tid, &addr),
        Err(NetError::Interrupted) => build_error(tid, ErrorCode::Interrupted),
        Err(NetError::Errno(code)) => build_error(tid, code),
    }
}

fn do_getsockname(
    backend: &NetBackend,
    tid: ThreadIdentifier,
    request: GetSockNameRequest,
) -> Message {
    trace!("networkd::getsockname(): tid={tid:?}, request={request:?}");
    match backend.getsockname(to_host_fd(request.sockfd)) {
        Ok(addr) => GetSockNameResponse::build(tid, &addr),
        Err(NetError::Interrupted) => build_error(tid, ErrorCode::Interrupted),
        Err(NetError::Errno(code)) => build_error(tid, code),
    }
}

fn do_accept(backend: &NetBackend, tid: ThreadIdentifier, request: AcceptSocketRequest) -> Message {
    trace!("networkd::accept(): tid={tid:?}, request={request:?}");
    match backend.accept(to_host_fd(request.sockfd)) {
        Ok((new_sockfd, addr)) => AcceptSocketResponse::build(tid, to_guest_fd(new_sockfd), &addr),
        Err(NetError::Interrupted) => build_error(tid, ErrorCode::Interrupted),
        Err(NetError::Errno(code)) => build_error(tid, code),
    }
}

fn do_recv(backend: &NetBackend, tid: ThreadIdentifier, request: ReceiveSocketRequest) -> Message {
    trace!("networkd::recv(): tid={tid:?}, request={request:?}");
    let recv_len: usize =
        core::cmp::min(ReceiveSocketResponse::BUFFER_SIZE, request.count as usize);
    let mut buffer: [u8; ReceiveSocketResponse::BUFFER_SIZE] =
        [0; ReceiveSocketResponse::BUFFER_SIZE];
    match backend.recv(to_host_fd(request.sockfd), &mut buffer, recv_len, request.flags) {
        Ok(count) => ReceiveSocketResponse::build(tid, count as u32, buffer),
        Err(NetError::Interrupted) => build_error(tid, ErrorCode::Interrupted),
        Err(NetError::Errno(code)) => build_error(tid, code),
    }
}

fn do_shutdown(
    backend: &NetBackend,
    tid: ThreadIdentifier,
    request: ShutdownSocketRequest,
) -> Message {
    trace!("networkd::shutdown(): tid={tid:?}, request={request:?}");
    match backend.shutdown(to_host_fd(request.sockfd), request.how) {
        Ok(()) => ShutdownSocketResponse::build(tid),
        Err(NetError::Interrupted) => build_error(tid, ErrorCode::Interrupted),
        Err(NetError::Errno(code)) => build_error(tid, code),
    }
}

fn do_send(backend: &NetBackend, tid: ThreadIdentifier, request: SendSocketRequest) -> Message {
    trace!("networkd::send(): tid={tid:?}, request={request:?}");
    match backend.send(
        to_host_fd(request.sockfd),
        &request.buffer,
        request.count as usize,
        request.flags,
    ) {
        Ok(count) => SendSocketResponse::build(tid, count as i32),
        Err(NetError::Interrupted) => build_error(tid, ErrorCode::Interrupted),
        Err(NetError::Errno(code)) => build_error(tid, code),
    }
}
