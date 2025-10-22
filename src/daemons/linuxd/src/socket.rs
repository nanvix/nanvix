// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::WorkerThreadError;
use ::core::{
    cmp,
    mem,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::{
    netinet_in::message_flags::{
        MSG_EOR,
        MSG_NOSIGNAL,
        MSG_OOB,
        MSG_PEEK,
        MSG_WAITALL,
    },
    sys_socket::{
        sockaddr,
        socklen_t,
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
    },
};
use ::syscall::{
    netinet::in_::Protocol,
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
        AddressFamily,
        Shutdown,
        SocketAddr,
        SocketType,
    },
};
use ::syslog::{
    debug,
    error,
    trace,
    warn,
};

//==================================================================================================
// do_socket
//==================================================================================================

pub fn do_socket(
    tid: ThreadIdentifier,
    request: CreateSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("socket(): tid={tid:?}, request={request:?}");

    let domain: LibcSocketDomain = match LibcSocketDomain::try_from(request.domain) {
        Ok(domain) => domain,
        Err(e) => return Ok(crate::build_error(tid, e.code)),
    };

    let typ: LibcSocketType = LibcSocketType::from(request.typ);

    let protocol: LibcSocketProtocol = LibcSocketProtocol::from(request.protocol);

    debug!(
        "libc::socket(): domain={:?}, type={:?}, protocol={protocol:?}",
        domain.inner(),
        typ.inner(),
    );
    match unsafe { libc::socket(domain.inner() as i32, typ.inner(), protocol.inner()) } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_socket(): worker thread interrupted while blocked on socket()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::socket(): failed with errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_socket(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(crate::build_error(tid, error))
        },
        sockfd => {
            debug!("libc::socket(): fd={sockfd:?}");
            Ok(CreateSocketResponse::build(tid, sockfd))
        },
    }
}

//==================================================================================================
// do_socketpair
//==================================================================================================

pub fn do_socketpair(
    tid: ThreadIdentifier,
    request: CreateSocketPairRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("socketpair(): tid={tid:?}, request={request:?}");

    let domain: LibcSocketDomain = match LibcSocketDomain::try_from(request.domain) {
        Ok(domain) => domain,
        Err(e) => return Ok(crate::build_error(tid, e.code)),
    };

    let typ: LibcSocketType = LibcSocketType::from(request.typ);

    let protocol: LibcSocketProtocol = LibcSocketProtocol::from(request.protocol);

    let mut sv: [libc::c_int; 2] = [0; 2];

    debug!(
        "libc::socketpair(): domain={:?}, type={:?}, protocol={protocol:?}",
        domain.inner(),
        typ.inner(),
    );
    match unsafe {
        libc::socketpair(domain.inner() as i32, typ.inner(), protocol.inner(), sv.as_mut_ptr())
    } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_socketpair(): worker thread interrupted while blocked on socketpair()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::socketpair(): failed with errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno)
                .unwrap_or_else(|_| panic!("unknown error code {errno:?}"));
            Ok(crate::build_error(tid, error))
        },
        _ => {
            debug!("libc::socketpair(): fds={sv:?}");
            Ok(CreateSocketPairResponse::build(tid, sv[0], sv[1]))
        },
    }
}

//==================================================================================================
// do_bind
//==================================================================================================

pub fn do_bind(
    tid: ThreadIdentifier,
    request: BindSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("bind(): tid={tid:?}, request={request:?}");

    let sockfd: i32 = request.sockfd;
    let sockaddr: LibcSocketAddress = match LibcSocketAddress::try_from(request.sockaddr) {
        Ok(sockaddr) => sockaddr,
        Err(e) => return Ok(crate::build_error(tid, e.code)),
    };
    let socklen: socklen_t = mem::size_of_val(&sockaddr) as socklen_t;

    debug!(
        "libc::bind(): sockfd={sockfd:?}, sockaddr.sa_family={:?}, sockaddr.sa_data={:?}, \
         socklen={socklen:?}",
        sockaddr.inner().sa_family,
        sockaddr.inner().sa_data,
    );
    match unsafe { libc::bind(sockfd, &sockaddr.inner() as *const libc::sockaddr, socklen) } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_bind(): worker thread interrupted while blocked on bind()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::bind(): failed with errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno)
                .unwrap_or_else(|_| panic!("unknown error code {errno:?}"));
            Ok(crate::build_error(tid, error))
        },
        _ => Ok(BindSocketResponse::build(tid)),
    }
}

//==================================================================================================
// do_connect
//==================================================================================================

pub fn do_connect(
    tid: ThreadIdentifier,
    request: ConnectSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("connect(): tid={tid:?}, request={request:?}");

    let sockfd: libc::c_int = request.sockfd;
    let sockaddr: LibcSocketAddress = match LibcSocketAddress::try_from(request.sockaddr) {
        Ok(sockaddr) => sockaddr,
        Err(e) => return Ok(crate::build_error(tid, e.code)),
    };
    let socklen: socklen_t = request.socklen;

    debug!(
        "libc::connect(): sockfd={sockfd:?}, sockaddr.sa_family={:?}, sockaddr.sa_data={:?}, \
         socklen={socklen:?}",
        sockaddr.inner().sa_family,
        sockaddr.inner().sa_data,
    );

    match unsafe {
        libc::connect(
            sockfd,
            &sockaddr.inner() as *const libc::sockaddr,
            socklen as libc::socklen_t,
        )
    } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_connect(): worker thread interrupted while blocked on connect()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::connect(): failed with errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno)
                .unwrap_or_else(|_| panic!("unknown error code {errno:?}"));
            Ok(crate::build_error(tid, error))
        },
        _ => Ok(ConnectSocketResponse::build(tid)),
    }
}

//==================================================================================================
// do_listen
//==================================================================================================

pub fn do_listen(
    tid: ThreadIdentifier,
    request: ListenSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("listen(): tid={tid:?}, request={request:?}");

    let sockfd: i32 = request.sockfd;
    let backlog: i32 = request.backlog;

    debug!("libc::listen(): sockfd={sockfd:?}, backlog={backlog:?}");
    match unsafe { libc::listen(sockfd, backlog) } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_listen(): worker thread interrupted while blocked on listen()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::listen(): failed with errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno)
                .unwrap_or_else(|_| panic!("unknown error code {errno:?}"));
            Ok(crate::build_error(tid, error))
        },
        _ => Ok(ListenSocketResponse::build(tid)),
    }
}

//==================================================================================================
// do_getpeername
//==================================================================================================

pub fn do_getpeername(
    tid: ThreadIdentifier,
    request: GetPeerNameRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("getpeername(): tid={tid:?}, request={request:?}");

    let sockfd: libc::c_int = request.sockfd;
    let mut address: libc::sockaddr = unsafe { core::mem::zeroed() };
    let mut address_len: libc::socklen_t =
        core::mem::size_of::<libc::sockaddr>() as libc::socklen_t;

    debug!("libc::getpeername(): sockfd={sockfd:?}");
    match unsafe { libc::getpeername(sockfd, &mut address, &mut address_len) } {
        -1 => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!(
                    "do_getpeername(): worker thread interrupted while blocked on getpeername()"
                );
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::getpeername(): failed with errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno)
                .unwrap_or_else(|_| panic!("unknown error code {errno:?}"));
            Ok(crate::build_error(tid, error))
        },
        _ => {
            let sockaddr: sockaddr = sockaddr {
                sa_len: address_len as u8,
                sa_family: address.sa_family as u8,
                sa_data: unsafe { core::mem::transmute::<[i8; 14], [u8; 14]>(address.sa_data) },
            };
            Ok(GetPeerNameResponse::build(tid, &sockaddr))
        },
    }
}

//==================================================================================================
// do_getsockname
//==================================================================================================

pub fn do_getsockname(
    tid: ThreadIdentifier,
    request: GetSockNameRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("getsockname(): tid={tid:?}, request={request:?}");

    let sockfd: libc::c_int = request.sockfd;
    let mut address: libc::sockaddr = unsafe { core::mem::zeroed() };
    let mut address_len: libc::socklen_t =
        core::mem::size_of::<libc::sockaddr>() as libc::socklen_t;

    debug!("libc::getsockname(): sockfd={sockfd:?}");
    match unsafe { libc::getsockname(sockfd, &mut address, &mut address_len) } {
        -1 => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!(
                    "do_getsockname(): worker thread interrupted while blocked on getsockname()"
                );
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::getsockname(): failed with errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno)
                .unwrap_or_else(|_| panic!("unknown error code {errno:?}"));
            error!("libc::getsockname(): {error:?}");
            Ok(crate::build_error(tid, error))
        },
        _ => {
            let sockaddr: sockaddr = sockaddr {
                sa_len: address_len as u8,
                sa_family: address.sa_family as u8,
                sa_data: unsafe { core::mem::transmute::<[i8; 14], [u8; 14]>(address.sa_data) },
            };
            Ok(GetSockNameResponse::build(tid, &sockaddr))
        },
    }
}

//==================================================================================================
// do_accept
//==================================================================================================

pub fn do_accept(
    tid: ThreadIdentifier,
    request: AcceptSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("accept(): tid={tid:?}, request={request:?}");

    let sockfd: i32 = request.sockfd;
    let mut address: libc::sockaddr = unsafe { core::mem::zeroed() };
    let mut address_len: libc::socklen_t =
        core::mem::size_of::<libc::sockaddr>() as libc::socklen_t;

    debug!("libc::accept(): sockfd={sockfd:?}");
    match unsafe { libc::accept(sockfd, &mut address, &mut address_len) } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_accept(): worker thread interrupted while blocked on accept()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::accept(): failed with errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno)
                .unwrap_or_else(|_| panic!("unknown error code {errno:?}"));
            Ok(crate::build_error(tid, error))
        },
        sockfd => {
            let sockaddr: sockaddr = sockaddr {
                sa_len: address_len as u8,
                sa_family: address.sa_family as u8,
                sa_data: unsafe { core::mem::transmute::<[i8; 14], [u8; 14]>(address.sa_data) },
            };
            Ok(AcceptSocketResponse::build(tid, sockfd, &sockaddr))
        },
    }
}

//==================================================================================================
// do_recv
//==================================================================================================

pub fn do_recv(
    tid: ThreadIdentifier,
    request: ReceiveSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("recv(): tid={tid:?}, request={request:?}");

    let sockfd: i32 = request.sockfd;
    let flags: LibcMessageFlags = match LibcMessageFlags::try_from(request.flags) {
        Ok(flags) => flags,
        Err(e) => return Ok(crate::build_error(tid, e.code)),
    };

    let recv_len: usize = cmp::min(ReceiveSocketResponse::BUFFER_SIZE, request.count as usize);

    let mut buffer: [u8; ReceiveSocketResponse::BUFFER_SIZE] =
        [0; ReceiveSocketResponse::BUFFER_SIZE];

    debug!("libc::recv(): sockfd={sockfd:?}, flags={:?}", flags.inner());
    match unsafe {
        libc::recv(sockfd, buffer.as_mut_ptr() as *mut libc::c_void, recv_len, flags.inner())
    } {
        count if count >= 0 => {
            debug!("libc::recv(): count={count:?}");
            Ok(ReceiveSocketResponse::build(tid, count as c_size_t, buffer))
        },
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_recv(): worker thread interrupted while blocked on recv()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::recv(): failed with errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno)
                .unwrap_or_else(|_| panic!("unknown error code {errno:?}"));
            Ok(crate::build_error(tid, error))
        },
        _ => unreachable!("libc::recv() returned invalid value"),
    }
}

//==================================================================================================
// do_shutdown
//==================================================================================================

pub fn do_shutdown(
    tid: ThreadIdentifier,
    request: ShutdownSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("shutdown(): tid={tid:?}, request={request:?}");

    let sockfd: i32 = request.sockfd;
    let how: LibcShutdownReason = LibcShutdownReason::from(request.how);

    debug!("libc::shutdown(): sockfd={sockfd:?}, how={:?}", how.inner());
    match unsafe { libc::shutdown(sockfd, how.inner()) } {
        0 => Ok(ShutdownSocketResponse::build(tid)),
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_shutdown(): worker thread interrupted while blocked on shutdown()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::shutdown(): failed with errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno)
                .unwrap_or_else(|_| panic!("unknown error code {errno:?}"));
            Ok(crate::build_error(tid, error))
        },
        ret => unreachable!("libc::shutdown() returned invalid value {ret:?}"),
    }
}

//==================================================================================================
// do_send
//==================================================================================================

pub fn do_send(
    tid: ThreadIdentifier,
    request: SendSocketRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("send(): tid={tid:?}, request={request:?}");

    let sockfd: i32 = request.sockfd;
    let count: c_size_t = request.count;
    let flags: LibcMessageFlags = match LibcMessageFlags::try_from(request.flags) {
        Ok(flags) => flags,
        Err(e) => return Ok(crate::build_error(tid, e.code)),
    };
    let buffer: [u8; SendSocketRequest::BUFFER_SIZE] = request.buffer;

    debug!(
        "libc::send(): sockfd={sockfd:?}, count={count:?}, flags={:?}, buffer={buffer:?}",
        flags.inner(),
    );
    match unsafe {
        libc::send(sockfd, buffer.as_ptr() as *const libc::c_void, count as usize, flags.inner())
    } {
        count if count >= 0 => {
            debug!("libc::send(): count={count:?}");
            Ok(SendSocketResponse::build(tid, count as c_ssize_t))
        },
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_send(): worker thread interrupted while blocked on send()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::send(): failed with errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno)
                .unwrap_or_else(|_| panic!("unknown error code {errno:?}"));
            Ok(crate::build_error(tid, error))
        },
        _ => unreachable!("libc::send() returned invalid value"),
    }
}

//==================================================================================================

struct LibcSocketDomain(libc::sa_family_t);

impl LibcSocketDomain {
    fn inner(&self) -> libc::sa_family_t {
        self.0
    }

    fn try_from(domain: AddressFamily) -> Result<Self, Error> {
        match domain {
            AddressFamily::Inet => Ok(Self(libc::AF_INET as libc::sa_family_t)),
            AddressFamily::Inet6 => Ok(Self(libc::AF_INET6 as libc::sa_family_t)),
            AddressFamily::Unix => Ok(Self(libc::AF_UNIX as libc::sa_family_t)),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid socket domain")),
        }
    }
}

struct LibcSocketType(libc::c_int);

impl LibcSocketType {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn from(type_: SocketType) -> Self {
        match type_ {
            SocketType::Datagram => Self(libc::SOCK_DGRAM),
            SocketType::Stream => Self(libc::SOCK_STREAM),
            SocketType::Raw => Self(libc::SOCK_RAW),
            SocketType::SeqPacket => Self(libc::SOCK_SEQPACKET),
        }
    }
}

#[derive(Debug)]
struct LibcSocketProtocol(libc::c_int);

impl LibcSocketProtocol {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn from(protocol: Protocol) -> Self {
        match protocol {
            Protocol::Ip => Self(libc::IPPROTO_IP),
            Protocol::Tcp => Self(libc::IPPROTO_TCP),
            Protocol::Udp => Self(libc::IPPROTO_UDP),
        }
    }
}

struct LibcSocketAddress(libc::sockaddr);

impl LibcSocketAddress {
    fn inner(&self) -> libc::sockaddr {
        self.0
    }
}

impl TryFrom<sockaddr> for LibcSocketAddress {
    type Error = Error;

    fn try_from(sockaddr: sockaddr) -> Result<Self, Self::Error> {
        let domain: i32 = sockaddr.sa_family.into();
        let domain: AddressFamily = match AddressFamily::try_from(domain) {
            Ok(domain) => domain,
            Err(_error) => {
                return Err(Error::new(
                    ErrorCode::InvalidArgument,
                    "failed to convert socket address",
                ))
            },
        };
        Ok(Self(libc::sockaddr {
            sa_family: LibcSocketDomain::try_from(domain)?.inner(),
            sa_data: unsafe { core::mem::transmute::<[u8; 14], [i8; 14]>(sockaddr.sa_data) },
        }))
    }
}

impl TryFrom<SocketAddr> for LibcSocketAddress {
    type Error = Error;

    fn try_from(sockaddr: SocketAddr) -> Result<Self, Self::Error> {
        let sockaddr: sockaddr = sockaddr::from(&sockaddr);
        LibcSocketAddress::try_from(sockaddr)
    }
}

struct LibcShutdownReason(libc::c_int);

impl LibcShutdownReason {
    fn inner(&self) -> libc::c_int {
        self.0
    }
}

impl From<Shutdown> for LibcShutdownReason {
    fn from(how: Shutdown) -> Self {
        match how {
            Shutdown::Read => Self(libc::SHUT_RD),
            Shutdown::Write => Self(libc::SHUT_WR),
            Shutdown::ReadWrite => Self(libc::SHUT_RDWR),
        }
    }
}

struct LibcMessageFlags(libc::c_int);

impl LibcMessageFlags {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn try_from(flags: i32) -> Result<Self, Error> {
        let mut flags = flags;
        let mut libc_flags = 0;

        let flag_mappings: [(i32, libc::c_int); 5] = [
            (MSG_PEEK, libc::MSG_PEEK),
            (MSG_OOB, libc::MSG_OOB),
            (MSG_WAITALL, libc::MSG_WAITALL),
            (MSG_EOR, libc::MSG_EOR),
            (MSG_NOSIGNAL, libc::MSG_NOSIGNAL),
        ];

        for (posix_flag, libc_flag) in &flag_mappings {
            if flags & posix_flag != 0 {
                libc_flags |= libc_flag;
                flags &= !posix_flag;
            }
        }

        if flags != 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid message flags"));
        }

        Ok(Self(libc_flags))
    }
}
