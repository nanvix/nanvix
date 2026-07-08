// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::WorkerThreadError,
    syscalls::SyscallTable,
};
use ::log::trace;
use ::net_backend::error::NetError;
use ::sys::{
    error::ErrorCode,
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::syscall::sys::socket::message::{
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
};

//==================================================================================================
// do_socket
//==================================================================================================

pub fn do_socket<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: CreateSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("socket(): tid={tid:?}, request={request:?}");

    let net_backend = match &syscall_table.net_backend {
        Some(backend) => backend,
        None => return Ok(crate::build_error(tid, ErrorCode::OperationNotSupported)),
    };

    match net_backend.socket(request.domain, request.typ, request.protocol) {
        Ok(sockfd) => Ok(CreateSocketResponse::build(tid, sockfd)),
        Err(NetError::Interrupted) => Err(WorkerThreadError::Interrupted),
        Err(NetError::Errno(code)) => Ok(crate::build_error(tid, code)),
    }
}

//==================================================================================================
// do_socketpair
//==================================================================================================

pub fn do_socketpair<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: CreateSocketPairRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("socketpair(): tid={tid:?}, request={request:?}");

    let net_backend = match &syscall_table.net_backend {
        Some(backend) => backend,
        None => return Ok(crate::build_error(tid, ErrorCode::OperationNotSupported)),
    };

    match net_backend.socketpair(request.domain, request.typ, request.protocol) {
        Ok((fd0, fd1)) => Ok(CreateSocketPairResponse::build(tid, fd0, fd1)),
        Err(NetError::Interrupted) => Err(WorkerThreadError::Interrupted),
        Err(NetError::Errno(code)) => Ok(crate::build_error(tid, code)),
    }
}

//==================================================================================================
// do_bind
//==================================================================================================

pub fn do_bind<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: BindSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("bind(): tid={tid:?}, request={request:?}");

    let net_backend = match &syscall_table.net_backend {
        Some(backend) => backend,
        None => return Ok(crate::build_error(tid, ErrorCode::OperationNotSupported)),
    };

    match net_backend.bind(request.sockfd, &request.sockaddr) {
        Ok(()) => Ok(BindSocketResponse::build(tid)),
        Err(NetError::Interrupted) => Err(WorkerThreadError::Interrupted),
        Err(NetError::Errno(code)) => Ok(crate::build_error(tid, code)),
    }
}

//==================================================================================================
// do_connect
//==================================================================================================

pub fn do_connect<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: ConnectSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("connect(): tid={tid:?}, request={request:?}");

    let net_backend = match &syscall_table.net_backend {
        Some(backend) => backend,
        None => return Ok(crate::build_error(tid, ErrorCode::OperationNotSupported)),
    };

    match net_backend.connect(request.sockfd, &request.sockaddr, request.socklen) {
        Ok(()) => Ok(ConnectSocketResponse::build(tid)),
        Err(NetError::Interrupted) => Err(WorkerThreadError::Interrupted),
        Err(NetError::Errno(code)) => Ok(crate::build_error(tid, code)),
    }
}

//==================================================================================================
// do_listen
//==================================================================================================

pub fn do_listen<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: ListenSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("listen(): tid={tid:?}, request={request:?}");

    let net_backend = match &syscall_table.net_backend {
        Some(backend) => backend,
        None => return Ok(crate::build_error(tid, ErrorCode::OperationNotSupported)),
    };

    match net_backend.listen(request.sockfd, request.backlog) {
        Ok(()) => Ok(ListenSocketResponse::build(tid)),
        Err(NetError::Interrupted) => Err(WorkerThreadError::Interrupted),
        Err(NetError::Errno(code)) => Ok(crate::build_error(tid, code)),
    }
}

//==================================================================================================
// do_getpeername
//==================================================================================================

pub fn do_getpeername<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: GetPeerNameRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("getpeername(): tid={tid:?}, request={request:?}");

    let net_backend = match &syscall_table.net_backend {
        Some(backend) => backend,
        None => return Ok(crate::build_error(tid, ErrorCode::OperationNotSupported)),
    };

    match net_backend.getpeername(request.sockfd) {
        Ok(addr) => Ok(GetPeerNameResponse::build(tid, &addr)),
        Err(NetError::Interrupted) => Err(WorkerThreadError::Interrupted),
        Err(NetError::Errno(code)) => Ok(crate::build_error(tid, code)),
    }
}

//==================================================================================================
// do_getsockname
//==================================================================================================

pub fn do_getsockname<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: GetSockNameRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("getsockname(): tid={tid:?}, request={request:?}");

    let net_backend = match &syscall_table.net_backend {
        Some(backend) => backend,
        None => return Ok(crate::build_error(tid, ErrorCode::OperationNotSupported)),
    };

    match net_backend.getsockname(request.sockfd) {
        Ok(addr) => Ok(GetSockNameResponse::build(tid, &addr)),
        Err(NetError::Interrupted) => Err(WorkerThreadError::Interrupted),
        Err(NetError::Errno(code)) => Ok(crate::build_error(tid, code)),
    }
}

//==================================================================================================
// do_accept
//==================================================================================================

pub fn do_accept<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: AcceptSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("accept(): tid={tid:?}, request={request:?}");

    let net_backend = match &syscall_table.net_backend {
        Some(backend) => backend,
        None => return Ok(crate::build_error(tid, ErrorCode::OperationNotSupported)),
    };

    match net_backend.accept(request.sockfd) {
        Ok((new_sockfd, addr)) => Ok(AcceptSocketResponse::build(tid, new_sockfd, &addr)),
        Err(NetError::Interrupted) => Err(WorkerThreadError::Interrupted),
        Err(NetError::Errno(code)) => Ok(crate::build_error(tid, code)),
    }
}

//==================================================================================================
// do_recv
//==================================================================================================

pub fn do_recv<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: ReceiveSocketRequest,
) -> Result<(Message, Vec<u8>), WorkerThreadError> {
    trace!("recv(): tid={tid:?}, request={request:?}");

    let net_backend = match &syscall_table.net_backend {
        Some(backend) => backend,
        None => return Ok((crate::build_error(tid, ErrorCode::OperationNotSupported), Vec::new())),
    };

    let recv_len: usize =
        core::cmp::min(ReceiveSocketResponse::MAX_DATA_SIZE, request.count as usize);
    let mut buffer: Vec<u8> = vec![0u8; recv_len];

    match net_backend.recv(request.sockfd, &mut buffer, recv_len, request.flags) {
        Ok(count) => {
            // Trim the buffer to the bytes actually received so the bulk transfer carries only
            // the received payload.
            buffer.truncate(count as usize);
            Ok((ReceiveSocketResponse::build(tid, count as u32), buffer))
        },
        Err(NetError::Interrupted) => Err(WorkerThreadError::Interrupted),
        Err(NetError::Errno(code)) => Ok((crate::build_error(tid, code), Vec::new())),
    }
}

//==================================================================================================
// do_shutdown
//==================================================================================================

pub fn do_shutdown<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: ShutdownSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("shutdown(): tid={tid:?}, request={request:?}");

    let net_backend = match &syscall_table.net_backend {
        Some(backend) => backend,
        None => return Ok(crate::build_error(tid, ErrorCode::OperationNotSupported)),
    };

    match net_backend.shutdown(request.sockfd, request.how) {
        Ok(()) => Ok(ShutdownSocketResponse::build(tid)),
        Err(NetError::Interrupted) => Err(WorkerThreadError::Interrupted),
        Err(NetError::Errno(code)) => Ok(crate::build_error(tid, code)),
    }
}

//==================================================================================================
// do_send
//==================================================================================================

pub fn do_send<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: SendSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("send(): tid={tid:?}, request={request:?}");

    let net_backend = match &syscall_table.net_backend {
        Some(backend) => backend,
        None => return Ok(crate::build_error(tid, ErrorCode::OperationNotSupported)),
    };

    match net_backend.send(request.sockfd, &request.buffer, request.count as usize, request.flags) {
        Ok(count) => Ok(SendSocketResponse::build(tid, count as i32)),
        Err(NetError::Interrupted) => Err(WorkerThreadError::Interrupted),
        Err(NetError::Errno(code)) => Ok(crate::build_error(tid, code)),
    }
}
