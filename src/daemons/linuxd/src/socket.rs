// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::linuxd::sys::socket::message::{
    CreateSocketRequest,
    CreateSocketResponse,
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
