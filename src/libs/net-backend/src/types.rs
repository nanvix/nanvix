// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::platform::{
    message_flag_mappings,
    sa_data_from_u8,
    shutdown_to_platform,
    to_sa_family,
    SaFamilyT,
    AF_INET,
    AF_INET6,
    AF_UNIX,
    IPPROTO_IP,
    IPPROTO_TCP,
    IPPROTO_UDP,
    SOCK_DGRAM,
    SOCK_RAW,
    SOCK_SEQPACKET,
    SOCK_STREAM,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::sys_socket::sockaddr;
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

pub(crate) struct LibcSocketDomain(SaFamilyT);

impl LibcSocketDomain {
    pub(crate) fn inner(&self) -> SaFamilyT {
        self.0
    }

    pub(crate) fn try_from_nanvix(domain: AddressFamily) -> Result<Self, Error> {
        match domain {
            AddressFamily::Inet => Ok(Self(AF_INET as SaFamilyT)),
            AddressFamily::Inet6 => Ok(Self(AF_INET6 as SaFamilyT)),
            AddressFamily::Unix => Ok(Self(AF_UNIX as SaFamilyT)),
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
            SocketType::Datagram => Self(SOCK_DGRAM),
            SocketType::Stream => Self(SOCK_STREAM),
            SocketType::Raw => Self(SOCK_RAW),
            SocketType::SeqPacket => Self(SOCK_SEQPACKET),
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
            Protocol::Ip => Self(IPPROTO_IP),
            Protocol::Tcp => Self(IPPROTO_TCP),
            Protocol::Udp => Self(IPPROTO_UDP),
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
            sa_family: to_sa_family(LibcSocketDomain::try_from_nanvix(domain)?.inner()),
            sa_data: sa_data_from_u8(sockaddr.sa_data),
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
        Self(shutdown_to_platform(how))
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

        let (flag_mappings, strip_flags) = message_flag_mappings();

        for &(posix_flag, libc_flag) in flag_mappings {
            if flags & posix_flag != 0 {
                libc_flags |= libc_flag;
                flags &= !posix_flag;
            }
        }

        // Strip platform-unsupported flags silently.
        for &strip_flag in strip_flags {
            flags &= !strip_flag;
        }

        if flags != 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid message flags"));
        }

        Ok(Self(libc_flags))
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod test;
