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
mod test {
    use super::*;
    use crate::platform::{
        AF_INET,
        AF_INET6,
        AF_UNIX,
        IPPROTO_IP,
        IPPROTO_TCP,
        IPPROTO_UDP,
        SHUT_RD,
        SHUT_RDWR,
        SHUT_WR,
        SOCK_DGRAM,
        SOCK_RAW,
        SOCK_SEQPACKET,
        SOCK_STREAM,
    };
    use ::sysapi::netinet_in::message_flags::{
        MSG_OOB,
        MSG_PEEK,
    };

    // ---- LibcSocketDomain -----------------------------------------------------------------------

    /// Tests that `AddressFamily::Inet` maps to the platform `AF_INET` constant.
    #[test]
    fn try_from_nanvix_inet_domain() {
        let domain: LibcSocketDomain = LibcSocketDomain::try_from_nanvix(AddressFamily::Inet)
            .expect("Inet domain conversion should succeed");
        assert_eq!(domain.inner() as i32, AF_INET, "Inet should map to AF_INET");
    }

    /// Tests that `AddressFamily::Inet6` maps to the platform `AF_INET6` constant.
    #[test]
    fn try_from_nanvix_inet6_domain() {
        let domain: LibcSocketDomain = LibcSocketDomain::try_from_nanvix(AddressFamily::Inet6)
            .expect("Inet6 domain conversion should succeed");
        assert_eq!(domain.inner() as i32, AF_INET6, "Inet6 should map to AF_INET6");
    }

    /// Tests that `AddressFamily::Unix` maps to the platform `AF_UNIX` constant.
    #[test]
    fn try_from_nanvix_unix_domain() {
        let domain: LibcSocketDomain = LibcSocketDomain::try_from_nanvix(AddressFamily::Unix)
            .expect("Unix domain conversion should succeed");
        assert_eq!(domain.inner() as i32, AF_UNIX, "Unix should map to AF_UNIX");
    }

    /// Tests that `AddressFamily::Unspec` is rejected with `InvalidArgument`.
    #[test]
    fn try_from_nanvix_unspec_domain_rejected() {
        let result: Result<LibcSocketDomain, _> =
            LibcSocketDomain::try_from_nanvix(AddressFamily::Unspec);
        assert!(result.is_err(), "Unspec should be rejected");
    }

    // ---- LibcSocketType -------------------------------------------------------------------------

    /// Tests that `SocketType::Stream` maps to `SOCK_STREAM`.
    #[test]
    fn from_nanvix_stream_type() {
        let typ: LibcSocketType = LibcSocketType::from_nanvix(SocketType::Stream);
        assert_eq!(typ.inner(), SOCK_STREAM, "Stream should map to SOCK_STREAM");
    }

    /// Tests that `SocketType::Datagram` maps to `SOCK_DGRAM`.
    #[test]
    fn from_nanvix_datagram_type() {
        let typ: LibcSocketType = LibcSocketType::from_nanvix(SocketType::Datagram);
        assert_eq!(typ.inner(), SOCK_DGRAM, "Datagram should map to SOCK_DGRAM");
    }

    /// Tests that `SocketType::Raw` maps to `SOCK_RAW`.
    #[test]
    fn from_nanvix_raw_type() {
        let typ: LibcSocketType = LibcSocketType::from_nanvix(SocketType::Raw);
        assert_eq!(typ.inner(), SOCK_RAW, "Raw should map to SOCK_RAW");
    }

    /// Tests that `SocketType::SeqPacket` maps to `SOCK_SEQPACKET`.
    #[test]
    fn from_nanvix_seqpacket_type() {
        let typ: LibcSocketType = LibcSocketType::from_nanvix(SocketType::SeqPacket);
        assert_eq!(typ.inner(), SOCK_SEQPACKET, "SeqPacket should map to SOCK_SEQPACKET");
    }

    // ---- LibcSocketProtocol ---------------------------------------------------------------------

    /// Tests that `Protocol::Ip` maps to `IPPROTO_IP`.
    #[test]
    fn from_nanvix_ip_protocol() {
        let proto: LibcSocketProtocol = LibcSocketProtocol::from_nanvix(Protocol::Ip);
        assert_eq!(proto.inner(), IPPROTO_IP, "Ip should map to IPPROTO_IP");
    }

    /// Tests that `Protocol::Tcp` maps to `IPPROTO_TCP`.
    #[test]
    fn from_nanvix_tcp_protocol() {
        let proto: LibcSocketProtocol = LibcSocketProtocol::from_nanvix(Protocol::Tcp);
        assert_eq!(proto.inner(), IPPROTO_TCP, "Tcp should map to IPPROTO_TCP");
    }

    /// Tests that `Protocol::Udp` maps to `IPPROTO_UDP`.
    #[test]
    fn from_nanvix_udp_protocol() {
        let proto: LibcSocketProtocol = LibcSocketProtocol::from_nanvix(Protocol::Udp);
        assert_eq!(proto.inner(), IPPROTO_UDP, "Udp should map to IPPROTO_UDP");
    }

    // ---- LibcSocketAddress ----------------------------------------------------------------------

    /// Tests that a valid `AF_INET` sockaddr converts successfully with sa_data preserved.
    #[test]
    fn try_from_valid_inet_sockaddr() {
        let sa_data: [u8; 14] = [0, 80, 127, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0];
        let addr: sockaddr = sockaddr {
            sa_len: core::mem::size_of::<sockaddr>() as u8,
            sa_family: 2, // AF_INET
            sa_data,
        };
        let libc_addr: LibcSocketAddress =
            LibcSocketAddress::try_from(addr).expect("valid inet sockaddr should convert");
        assert_eq!(
            libc_addr.inner().sa_family as i32 & 0xFF,
            AF_INET & 0xFF,
            "sa_family should match AF_INET"
        );
    }

    /// Tests that a sockaddr with an invalid address family is rejected.
    #[test]
    fn try_from_invalid_family_rejected() {
        let addr: sockaddr = sockaddr {
            sa_len: 0,
            sa_family: 255, // Invalid family.
            sa_data: [0; 14],
        };
        let result: Result<LibcSocketAddress, _> = LibcSocketAddress::try_from(addr);
        assert!(result.is_err(), "invalid address family should be rejected");
    }

    // ---- LibcShutdownReason ---------------------------------------------------------------------

    /// Tests that `Shutdown::Read` maps to `SHUT_RD`.
    #[test]
    fn shutdown_read_maps_to_shut_rd() {
        let reason: LibcShutdownReason = LibcShutdownReason::from(Shutdown::Read);
        assert_eq!(reason.inner(), SHUT_RD, "Read should map to SHUT_RD");
    }

    /// Tests that `Shutdown::Write` maps to `SHUT_WR`.
    #[test]
    fn shutdown_write_maps_to_shut_wr() {
        let reason: LibcShutdownReason = LibcShutdownReason::from(Shutdown::Write);
        assert_eq!(reason.inner(), SHUT_WR, "Write should map to SHUT_WR");
    }

    /// Tests that `Shutdown::ReadWrite` maps to `SHUT_RDWR`.
    #[test]
    fn shutdown_readwrite_maps_to_shut_rdwr() {
        let reason: LibcShutdownReason = LibcShutdownReason::from(Shutdown::ReadWrite);
        assert_eq!(reason.inner(), SHUT_RDWR, "ReadWrite should map to SHUT_RDWR");
    }

    // ---- LibcMessageFlags -----------------------------------------------------------------------

    /// Tests that zero flags convert to zero platform flags.
    #[test]
    fn message_flags_zero() {
        let flags: LibcMessageFlags =
            LibcMessageFlags::try_from_nanvix(0).expect("zero flags should succeed");
        assert_eq!(flags.inner(), 0, "zero input should produce zero output");
    }

    /// Tests that `MSG_PEEK` converts to the platform equivalent.
    #[test]
    fn message_flags_peek() {
        let flags: LibcMessageFlags =
            LibcMessageFlags::try_from_nanvix(MSG_PEEK).expect("MSG_PEEK should succeed");
        assert_ne!(flags.inner(), 0, "MSG_PEEK should produce a non-zero flag");
    }

    /// Tests that combined `MSG_PEEK | MSG_OOB` converts successfully.
    #[test]
    fn message_flags_combined() {
        let input: i32 = MSG_PEEK | MSG_OOB;
        let flags: LibcMessageFlags =
            LibcMessageFlags::try_from_nanvix(input).expect("combined flags should succeed");
        assert_ne!(flags.inner(), 0, "combined flags should produce non-zero output");
    }

    /// Tests that an unrecognized flag bit is rejected.
    #[test]
    fn message_flags_unknown_rejected() {
        let result: Result<LibcMessageFlags, _> = LibcMessageFlags::try_from_nanvix(0x4000_0000);
        assert!(result.is_err(), "unknown flag bits should be rejected");
    }
}
