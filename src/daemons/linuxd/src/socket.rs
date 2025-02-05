// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    cmp,
    mem,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};
use ::posix::sys::{
    socket::{
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
        sockaddr,
        socklen_t,
        Shutdown,
        SocketAddr,
    },
    types::{
        size_t,
        ssize_t,
    },
};

//==================================================================================================
// do_socket
//==================================================================================================

pub fn do_socket(pid: ProcessIdentifier, request: CreateSocketRequest) -> Message {
    trace!("socket(): pid={:?}, request={:?}", pid, request);

    let domain: LibcSocketDomain = match LibcSocketDomain::try_from(request.domain) {
        Ok(domain) => domain,
        Err(e) => return crate::build_error(pid, e.code),
    };

    let typ: LibcSocketType = match LibcSocketType::try_from(request.typ) {
        Ok(typ) => typ,
        Err(e) => return crate::build_error(pid, e.code),
    };

    let protocol: i32 = request.protocol;

    debug!(
        "libc::socket(): domain={:?}, type={:?}, protocol={:?}",
        domain.inner(),
        typ.inner(),
        protocol
    );
    match unsafe { libc::socket(domain.inner() as i32, typ.inner(), protocol) } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            let error: ErrorCode = ErrorCode::try_from(-errno)
                .unwrap_or_else(|_| panic!("unknown error code {:?}", errno));
            crate::build_error(pid, error)
        },
        sockfd => {
            debug!("libc::socket(): fd={:?}", sockfd);
            CreateSocketResponse::build(pid, sockfd)
        },
    }
}

//==================================================================================================
// do_socketpair
//==================================================================================================

pub fn do_socketpair(pid: ProcessIdentifier, request: CreateSocketPairRequest) -> Message {
    trace!("socketpair(): pid={:?}, request={:?}", pid, request);

    let domain: LibcSocketDomain = match LibcSocketDomain::try_from(request.domain) {
        Ok(domain) => domain,
        Err(e) => return crate::build_error(pid, e.code),
    };

    let typ: LibcSocketType = match LibcSocketType::try_from(request.typ) {
        Ok(typ) => typ,
        Err(e) => return crate::build_error(pid, e.code),
    };

    let protocol: libc::c_int = request.protocol;

    let mut sv: [libc::c_int; 2] = [0; 2];

    debug!(
        "libc::socketpair(): domain={:?}, type={:?}, protocol={:?}",
        domain.inner(),
        typ.inner(),
        protocol
    );
    match unsafe { libc::socketpair(domain.inner() as i32, typ.inner(), protocol, sv.as_mut_ptr()) }
    {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            let error: ErrorCode = ErrorCode::try_from(-errno)
                .unwrap_or_else(|_| panic!("unknown error code {:?}", errno));
            crate::build_error(pid, error)
        },
        _ => {
            debug!("libc::socketpair(): fds={:?}", sv);
            CreateSocketPairResponse::build(pid, sv[0], sv[1])
        },
    }
}

//==================================================================================================
// do_bind
//==================================================================================================

pub fn do_bind(pid: ProcessIdentifier, request: BindSocketRequest) -> Message {
    trace!("bind(): pid={:?}, request={:?}", pid, request);

    let sockfd: i32 = request.sockfd;
    let sockaddr: LibcSocketAddress = match LibcSocketAddress::try_from(request.sockaddr) {
        Ok(sockaddr) => sockaddr,
        Err(e) => return crate::build_error(pid, e.code),
    };
    let socklen: socklen_t = mem::size_of_val(&sockaddr) as socklen_t;

    debug!(
        "libc::bind(): sockfd={:?}, sockaddr.sa_family={:?}, sockaddr.sa_data={:?}, socklen={:?}",
        sockfd,
        sockaddr.inner().sa_family,
        sockaddr.inner().sa_data,
        socklen
    );
    match unsafe { libc::bind(sockfd, &sockaddr.inner() as *const libc::sockaddr, socklen) } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            let error: ErrorCode = ErrorCode::try_from(-errno)
                .unwrap_or_else(|_| panic!("unknown error code {:?}", errno));
            crate::build_error(pid, error)
        },
        _ => BindSocketResponse::build(pid),
    }
}

//==================================================================================================
// do_connect
//==================================================================================================

pub fn do_connect(pid: ProcessIdentifier, request: ConnectSocketRequest) -> Message {
    trace!("connect(): pid={:?}, request={:?}", pid, request);

    let sockfd: libc::c_int = request.sockfd;
    let sockaddr: LibcSocketAddress = match LibcSocketAddress::try_from(request.sockaddr) {
        Ok(sockaddr) => sockaddr,
        Err(e) => return crate::build_error(pid, e.code),
    };
    let socklen: socklen_t = request.socklen;

    debug!(
        "libc::connect(): sockfd={:?}, sockaddr.sa_family={:?}, sockaddr.sa_data={:?}, \
         socklen={:?}",
        sockfd,
        sockaddr.inner().sa_family,
        sockaddr.inner().sa_data,
        socklen
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
            let error: ErrorCode = ErrorCode::try_from(-errno)
                .unwrap_or_else(|_| panic!("unknown error code {:?}", errno));
            crate::build_error(pid, error)
        },
        sockfd => BindSocketResponse::build(pid, sockfd),
    }
}

//==================================================================================================
// do_listen
//==================================================================================================

pub fn do_listen(pid: ProcessIdentifier, request: ListenSocketRequest) -> Message {
    trace!("listen(): pid={:?}, request={:?}", pid, request);

    let sockfd: i32 = request.sockfd;
    let backlog: i32 = request.backlog;

    debug!("libc::listen(): sockfd={:?}, backlog={:?}", sockfd, backlog);
    match unsafe { libc::listen(sockfd, backlog) } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            let error: ErrorCode = ErrorCode::try_from(-errno)
                .unwrap_or_else(|_| panic!("unknown error code {:?}", errno));
            crate::build_error(pid, error)
        },
        _ => ListenSocketResponse::build(pid, 0),
    }
}

//==================================================================================================
// do_getpeername
//==================================================================================================

pub fn do_getpeername(pid: ProcessIdentifier, request: GetPeerNameRequest) -> Message {
    trace!("getpeername(): pid={:?}, request={:?}", pid, request);

    let sockfd: libc::c_int = request.sockfd;
    let mut address: libc::sockaddr = unsafe { core::mem::zeroed() };
    let mut address_len: libc::socklen_t =
        core::mem::size_of::<libc::sockaddr>() as libc::socklen_t;

    debug!("libc::getpeername(): sockfd={:?}", sockfd);
    match unsafe { libc::getpeername(sockfd, &mut address, &mut address_len) } {
        -1 => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };
            let error: ErrorCode = ErrorCode::try_from(-errno)
                .unwrap_or_else(|_| panic!("unknown error code {:?}", errno));
            crate::build_error(pid, error)
        },
        _ => {
            let sockaddr: sockaddr = sockaddr {
                sa_family: address.sa_family as u16,
                sa_data: unsafe { core::mem::transmute::<[i8; 14], [u8; 14]>(address.sa_data) },
            };
            let sockaddr: SocketAddr = sockaddr.into();
            GetPeerNameResponse::build(pid, sockaddr)
        },
    }
}

//==================================================================================================
// do_getsockname
//==================================================================================================

pub fn do_getsockname(pid: ProcessIdentifier, request: GetSockNameRequest) -> Message {
    trace!("getsockname(): pid={:?}, request={:?}", pid, request);

    let sockfd: libc::c_int = request.sockfd;
    let mut address: libc::sockaddr = unsafe { core::mem::zeroed() };
    let mut address_len: libc::socklen_t =
        core::mem::size_of::<libc::sockaddr>() as libc::socklen_t;

    debug!("libc::getsockname(): sockfd={:?}", sockfd);
    match unsafe { libc::getsockname(sockfd, &mut address, &mut address_len) } {
        -1 => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };
            let error: ErrorCode = ErrorCode::try_from(-errno)
                .unwrap_or_else(|_| panic!("unknown error code {:?}", errno));
            crate::build_error(pid, error)
        },
        _ => {
            let sockaddr: sockaddr = sockaddr {
                sa_family: address.sa_family as u16,
                sa_data: unsafe { core::mem::transmute::<[i8; 14], [u8; 14]>(address.sa_data) },
            };
            let sockaddr: SocketAddr = sockaddr.into();
            GetSockNameResponse::build(pid, sockaddr)
        },
    }
}

//==================================================================================================
// do_accept
//==================================================================================================

pub fn do_accept(pid: ProcessIdentifier, request: AcceptSocketRequest) -> Message {
    trace!("accept(): pid={:?}, request={:?}", pid, request);

    let sockfd: i32 = request.sockfd;
    let mut address: libc::sockaddr = unsafe { core::mem::zeroed() };
    let mut address_len: libc::socklen_t =
        core::mem::size_of::<libc::sockaddr>() as libc::socklen_t;

    debug!("libc::accept(): sockfd={:?}", sockfd);
    match unsafe { libc::accept(sockfd, &mut address, &mut address_len) } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            let error: ErrorCode = ErrorCode::try_from(-errno)
                .unwrap_or_else(|_| panic!("unknown error code {:?}", errno));
            crate::build_error(pid, error)
        },
        sockfd => {
            let sockaddr: sockaddr = sockaddr {
                sa_family: address.sa_family as u16,
                sa_data: unsafe { core::mem::transmute::<[i8; 14], [u8; 14]>(address.sa_data) },
            };
            let sockaddr: SocketAddr = sockaddr.into();
            AcceptSocketResponse::build(pid, sockfd, sockaddr)
        },
    }
}

//==================================================================================================
// do_recv
//==================================================================================================

pub fn do_recv(pid: ProcessIdentifier, request: ReceiveSocketRequest) -> Message {
    trace!("recv(): pid={:?}, request={:?}", pid, request);

    let sockfd: i32 = request.sockfd;
    let flags: LibcMessageFlags = match LibcMessageFlags::try_from(request.flags) {
        Ok(flags) => flags,
        Err(e) => return crate::build_error(pid, e.code),
    };

    let recv_len: usize = cmp::min(ReceiveSocketResponse::BUFFER_SIZE, request.count as usize);

    let mut buffer: [u8; ReceiveSocketResponse::BUFFER_SIZE] =
        [0; ReceiveSocketResponse::BUFFER_SIZE];

    debug!("libc::recv(): sockfd={:?}, flags={:?}", sockfd, flags.inner());
    match unsafe {
        libc::recv(sockfd, buffer.as_mut_ptr() as *mut libc::c_void, recv_len, flags.inner())
    } {
        count if count >= 0 => {
            debug!("libc::recv(): count={:?}", count);
            ReceiveSocketResponse::build(pid, count as size_t, buffer)
        },
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            let error: ErrorCode = ErrorCode::try_from(-errno)
                .unwrap_or_else(|_| panic!("unknown error code {:?}", errno));
            crate::build_error(pid, error)
        },
        _ => unreachable!("libc::recv() returned invalid value"),
    }
}

//==================================================================================================
// do_shutdown
//==================================================================================================

pub fn do_shutdown(pid: ProcessIdentifier, request: ShutdownSocketRequest) -> Message {
    trace!("shutdown(): pid={:?}, request={:?}", pid, request);

    let sockfd: i32 = request.sockfd;
    let how: LibcShutdownReason = LibcShutdownReason::from(request.how);

    debug!("libc::shutdown(): sockfd={:?}, how={:?}", sockfd, how.inner());
    match unsafe { libc::shutdown(sockfd, how.inner()) } {
        0 => ShutdownSocketResponse::build(pid),
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            let error: ErrorCode = ErrorCode::try_from(-errno)
                .unwrap_or_else(|_| panic!("unknown error code {:?}", errno));
            crate::build_error(pid, error)
        },
        ret => unreachable!("libc::shutdown() returned invalid value {:?}", ret),
    }
}

//==================================================================================================
// do_send
//==================================================================================================

pub fn do_send(pid: ProcessIdentifier, request: SendSocketRequest) -> Message {
    trace!("send(): pid={:?}, request={:?}", pid, request);

    let sockfd: i32 = request.sockfd;
    let count: size_t = request.count;
    let flags: LibcMessageFlags = match LibcMessageFlags::try_from(request.flags) {
        Ok(flags) => flags,
        Err(e) => return crate::build_error(pid, e.code),
    };
    let buffer: [u8; SendSocketRequest::BUFFER_SIZE] = request.buffer;

    debug!(
        "libc::send(): sockfd={:?}, count={:?}, flags={:?}, buffer={:?}",
        sockfd,
        count,
        flags.inner(),
        buffer
    );
    match unsafe {
        libc::send(sockfd, buffer.as_ptr() as *const libc::c_void, count as usize, flags.inner())
    } {
        count if count >= 0 => {
            debug!("libc::send(): count={:?}", count);
            SendSocketResponse::build(pid, count as ssize_t)
        },
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            let error: ErrorCode = ErrorCode::try_from(-errno)
                .unwrap_or_else(|_| panic!("unknown error code {:?}", errno));
            crate::build_error(pid, error)
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

    fn try_from(domain: i32) -> Result<Self, Error> {
        match domain as u16 {
            ::posix::sys::socket::AF_INET => Ok(Self(libc::AF_INET as libc::sa_family_t)),
            ::posix::sys::socket::AF_INET6 => Ok(Self(libc::AF_INET6 as libc::sa_family_t)),
            ::posix::sys::socket::AF_UNIX => Ok(Self(libc::AF_UNIX as libc::sa_family_t)),
            ::posix::sys::socket::AF_UNSPEC => Ok(Self(libc::AF_UNSPEC as libc::sa_family_t)),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid socket domain")),
        }
    }
}

struct LibcSocketType(libc::c_int);

impl LibcSocketType {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn try_from(type_: i32) -> Result<Self, Error> {
        match type_ {
            ::posix::sys::socket::SOCK_STREAM => Ok(Self(libc::SOCK_STREAM)),
            ::posix::sys::socket::SOCK_DGRAM => Ok(Self(libc::SOCK_DGRAM)),
            ::posix::sys::socket::SOCK_SEQPACKET => Ok(Self(libc::SOCK_SEQPACKET)),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid socket type")),
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
        Ok(Self(libc::sockaddr {
            sa_family: LibcSocketDomain::try_from(sockaddr.sa_family as i32)?.inner(),
            sa_data: unsafe { core::mem::transmute::<[u8; 14], [i8; 14]>(sockaddr.sa_data) },
        }))
    }
}

impl TryFrom<SocketAddr> for LibcSocketAddress {
    type Error = Error;

    fn try_from(sockaddr: SocketAddr) -> Result<Self, Self::Error> {
        let sockaddr: sockaddr = sockaddr.into();
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
            (::posix::sys::socket::MSG_PEEK, libc::MSG_PEEK),
            (::posix::sys::socket::MSG_OOB, libc::MSG_OOB),
            (::posix::sys::socket::MSG_WAITALL, libc::MSG_WAITALL),
            (::posix::sys::socket::MSG_EOR, libc::MSG_EOR),
            (::posix::sys::socket::MSG_NOSIGNAL, libc::MSG_NOSIGNAL),
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
