// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    netinet_in::message_flags::{
        MSG_EOR,
        MSG_NOSIGNAL,
        MSG_OOB,
        MSG_PEEK,
        MSG_WAITALL,
    },
    sys_socket::sockaddr,
};
use ::syscall::{
    netinet::in_::Protocol,
    sys::socket::{
        AddressFamily,
        Shutdown,
        SocketType,
    },
};

//==================================================================================================
// LibcSocketDomain
//==================================================================================================

pub(crate) struct LibcSocketDomain(libc::sa_family_t);

impl LibcSocketDomain {
    pub(crate) fn inner(&self) -> libc::sa_family_t {
        self.0
    }

    pub(crate) fn try_from_nanvix(domain: AddressFamily) -> Result<Self, Error> {
        match domain {
            AddressFamily::Inet => Ok(Self(libc::AF_INET as libc::sa_family_t)),
            AddressFamily::Inet6 => Ok(Self(libc::AF_INET6 as libc::sa_family_t)),
            AddressFamily::Unix => Ok(Self(libc::AF_UNIX as libc::sa_family_t)),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid socket domain")),
        }
    }
}

//==================================================================================================
// LibcSocketType
//==================================================================================================

pub(crate) struct LibcSocketType(libc::c_int);

impl LibcSocketType {
    pub(crate) fn inner(&self) -> libc::c_int {
        self.0
    }

    pub(crate) fn from_nanvix(type_: SocketType) -> Self {
        match type_ {
            SocketType::Datagram => Self(libc::SOCK_DGRAM),
            SocketType::Stream => Self(libc::SOCK_STREAM),
            SocketType::Raw => Self(libc::SOCK_RAW),
            SocketType::SeqPacket => Self(libc::SOCK_SEQPACKET),
        }
    }
}

//==================================================================================================
// LibcSocketProtocol
//==================================================================================================

#[derive(Debug)]
pub(crate) struct LibcSocketProtocol(libc::c_int);

impl LibcSocketProtocol {
    pub(crate) fn inner(&self) -> libc::c_int {
        self.0
    }

    pub(crate) fn from_nanvix(protocol: Protocol) -> Self {
        match protocol {
            Protocol::Ip => Self(libc::IPPROTO_IP),
            Protocol::Tcp => Self(libc::IPPROTO_TCP),
            Protocol::Udp => Self(libc::IPPROTO_UDP),
        }
    }
}

//==================================================================================================
// LibcSocketAddress
//==================================================================================================

pub(crate) struct LibcSocketAddress(libc::sockaddr);

impl LibcSocketAddress {
    pub(crate) fn inner(&self) -> libc::sockaddr {
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
            sa_family: LibcSocketDomain::try_from_nanvix(domain)?.inner(),
            sa_data: sockaddr.sa_data.map(|b| b as i8),
        }))
    }
}

//==================================================================================================
// LibcShutdownReason
//==================================================================================================

pub(crate) struct LibcShutdownReason(libc::c_int);

impl LibcShutdownReason {
    pub(crate) fn inner(&self) -> libc::c_int {
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

//==================================================================================================
// LibcMessageFlags
//==================================================================================================

pub(crate) struct LibcMessageFlags(libc::c_int);

impl LibcMessageFlags {
    pub(crate) fn inner(&self) -> libc::c_int {
        self.0
    }

    pub(crate) fn try_from_nanvix(flags: i32) -> Result<Self, Error> {
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
