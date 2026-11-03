// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Tests that Linux errno values that collide with unrelated Nanvix errno values are translated
/// by semantic name instead of by raw number.
#[cfg(target_os = "linux")]
#[test]
fn try_from_linux_errno_translates_socket_family() {
    assert_eq!(
        ErrorCode::try_from_linux_errno(libc::EINPROGRESS).unwrap(),
        ErrorCode::OperationInProgress
    );
    assert_eq!(
        ErrorCode::try_from_linux_errno(libc::EALREADY).unwrap(),
        ErrorCode::OperationAlreadyInProgress
    );
    assert_eq!(
        ErrorCode::try_from_linux_errno(libc::EISCONN).unwrap(),
        ErrorCode::TransportEndpointConnected
    );
}

/// Tests that unknown Linux errno values do not fall through into Nanvix errno numbering.
#[cfg(target_os = "linux")]
#[test]
fn try_from_linux_errno_rejects_unknown_values() {
    assert!(ErrorCode::try_from_linux_errno(41).is_err());
}

/// Tests that Winsock errno values are translated by semantic name.
#[cfg(target_os = "windows")]
#[test]
fn try_from_winsock_errno_translates_socket_family() {
    let cases: [(i32, ErrorCode); 12] = [
        (winsock_errno::WSAEAFNOSUPPORT.0, ErrorCode::AddressFamilyNotSupported),
        (winsock_errno::WSAEADDRINUSE.0, ErrorCode::AddressInUse),
        (winsock_errno::WSAEDESTADDRREQ.0, ErrorCode::DestinationAddressRequired),
        (winsock_errno::WSAEHOSTUNREACH.0, ErrorCode::HostUnreachable),
        (winsock_errno::WSAEISCONN.0, ErrorCode::TransportEndpointConnected),
        (winsock_errno::WSAENETDOWN.0, ErrorCode::NetworkDown),
        (winsock_errno::WSAENETRESET.0, ErrorCode::NetworkReset),
        (winsock_errno::WSAEPROTONOSUPPORT.0, ErrorCode::ProtocolNotSupported),
        (winsock_errno::WSAEPROTOTYPE.0, ErrorCode::BadProtocolType),
        (winsock_errno::WSAESHUTDOWN.0, ErrorCode::TransportEndpointShutdown),
        (winsock_errno::WSAESOCKTNOSUPPORT.0, ErrorCode::SocketTypeNotSupported),
        (winsock_errno::WSAETIMEDOUT.0, ErrorCode::OperationTimedOut),
    ];

    for (winsock_error, expected) in cases {
        assert_eq!(ErrorCode::try_from_winsock_errno(winsock_error).unwrap(), expected);
    }
}

/// Tests that `WSAEWOULDBLOCK` is connect-specific: generic I/O sees `TryAgain`, while
/// `connect()` sees `OperationInProgress`.
#[cfg(target_os = "windows")]
#[test]
fn try_from_winsock_connect_errno_handles_wouldblock() {
    assert_eq!(
        ErrorCode::try_from_winsock_errno(winsock_errno::WSAEWOULDBLOCK.0).unwrap(),
        ErrorCode::TryAgain
    );
    assert_eq!(
        ErrorCode::try_from_winsock_connect_errno(winsock_errno::WSAEWOULDBLOCK.0).unwrap(),
        ErrorCode::OperationInProgress
    );
    assert_eq!(
        ErrorCode::try_from_winsock_connect_errno(winsock_errno::WSAEINVAL.0).unwrap(),
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        ErrorCode::try_from_winsock_connect_errno(winsock_errno::WSAEALREADY.0).unwrap(),
        ErrorCode::OperationAlreadyInProgress
    );
}

/// Tests that unknown Winsock errno values do not fall through into Nanvix errno numbering.
#[cfg(target_os = "windows")]
#[test]
fn try_from_winsock_errno_rejects_unknown_values() {
    assert!(ErrorCode::try_from_winsock_errno(99999).is_err());
}
