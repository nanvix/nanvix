// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::linuxd::sys::socket::{
    message::{
        AcceptSocketRequest,
        AcceptSocketResponse,
        BindSocketRequest,
        BindSocketResponse,
        CreateSocketRequest,
        CreateSocketResponse,
        ListenSocketRequest,
        ListenSocketResponse,
        ShutdownSocketRequest,
        ShutdownSocketResponse,
    },
    sockaddr,
    socklen_t,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
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
// do_bind
//==================================================================================================

pub fn do_bind(pid: ProcessIdentifier, request: BindSocketRequest) -> Message {
    trace!("bind(): pid={:?}, request={:?}", pid, request);

    let sockfd: i32 = request.sockfd;
    let sockaddr: LibcSocketAddress = match LibcSocketAddress::try_from(request.sockaddr) {
        Ok(sockaddr) => sockaddr,
        Err(e) => return crate::build_error(pid, e.code),
    };
    let socklen: socklen_t = request.socklen;

    debug!(
        "libc::bind(): sockfd={:?}, sockaddr.sa_family={:?}, sockaddr.sa_data={:?}, socklen={:?}",
        sockfd,
        sockaddr.inner().sa_family,
        sockaddr.inner().sa_data,
        socklen
    );
    match unsafe {
        libc::bind(sockfd, &sockaddr.inner() as *const libc::sockaddr, socklen as libc::socklen_t)
    } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            let error: ErrorCode = ErrorCode::try_from(-errno)
                .unwrap_or_else(|_| panic!("unknown error code {:?}", errno));
            crate::build_error(pid, error)
        },
        _ => BindSocketResponse::build(pid, 0),
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
            let socklen: socklen_t = address_len as socklen_t;
            AcceptSocketResponse::build(pid, sockfd, sockaddr, socklen)
        },
    }
}

//==================================================================================================
// do_shutdown
//==================================================================================================

pub fn do_shutdown(pid: ProcessIdentifier, request: ShutdownSocketRequest) -> Message {
    trace!("shutdown(): pid={:?}, request={:?}", pid, request);

    let sockfd: i32 = request.sockfd;
    let how: LibcShutdownReason = match LibcShutdownReason::try_from(request.how) {
        Ok(how) => how,
        Err(e) => return crate::build_error(pid, e.code),
    };

    debug!("libc::shutdown(): sockfd={:?}, how={:?}", sockfd, how.inner());
    match unsafe { libc::shutdown(sockfd, how.inner()) } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            let error: ErrorCode = ErrorCode::try_from(-errno)
                .unwrap_or_else(|_| panic!("unknown error code {:?}", errno));
            crate::build_error(pid, error)
        },
        ret => ShutdownSocketResponse::build(pid, ret),
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
            linuxd::sys::socket::AF_INET => Ok(Self(libc::AF_INET as libc::sa_family_t)),
            linuxd::sys::socket::AF_INET6 => Ok(Self(libc::AF_INET6 as libc::sa_family_t)),
            linuxd::sys::socket::AF_UNIX => Ok(Self(libc::AF_UNIX as libc::sa_family_t)),
            linuxd::sys::socket::AF_UNSPEC => Ok(Self(libc::AF_UNSPEC as libc::sa_family_t)),
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
            linuxd::sys::socket::SOCK_STREAM => Ok(Self(libc::SOCK_STREAM)),
            linuxd::sys::socket::SOCK_DGRAM => Ok(Self(libc::SOCK_DGRAM)),
            linuxd::sys::socket::SOCK_SEQPACKET => Ok(Self(libc::SOCK_SEQPACKET)),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid socket type")),
        }
    }
}

struct LibcSocketAddress(libc::sockaddr);

impl LibcSocketAddress {
    fn inner(&self) -> libc::sockaddr {
        self.0
    }

    fn try_from(sockaddr: sockaddr) -> Result<Self, Error> {
        Ok(Self(libc::sockaddr {
            sa_family: LibcSocketDomain::try_from(sockaddr.sa_family as i32)?.inner(),
            sa_data: unsafe { core::mem::transmute::<[u8; 14], [i8; 14]>(sockaddr.sa_data) },
        }))
    }
}

struct LibcShutdownReason(libc::c_int);

impl LibcShutdownReason {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn try_from(how: i32) -> Result<Self, Error> {
        match how {
            linuxd::sys::socket::SHUT_RD => Ok(Self(libc::SHUT_RD)),
            linuxd::sys::socket::SHUT_WR => Ok(Self(libc::SHUT_WR)),
            linuxd::sys::socket::SHUT_RDWR => Ok(Self(libc::SHUT_RDWR)),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid shutdown reason")),
        }
    }
}
