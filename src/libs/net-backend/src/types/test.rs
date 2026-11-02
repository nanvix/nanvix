// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
