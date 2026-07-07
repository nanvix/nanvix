// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Unix Platform Implementation
//==================================================================================================

//! Unix-specific networking primitives using libc.

//==================================================================================================
// Type Aliases
//==================================================================================================

pub(crate) type SaFamilyT = libc::sa_family_t;
pub(crate) type SocklenT = libc::socklen_t;
pub(crate) type RawSocket = libc::c_int;

//==================================================================================================
// Socket Constants
//==================================================================================================

pub(crate) const AF_INET: libc::c_int = libc::AF_INET;
pub(crate) const AF_INET6: libc::c_int = libc::AF_INET6;
pub(crate) const AF_UNIX: libc::c_int = libc::AF_UNIX;
pub(crate) const SOCK_STREAM: libc::c_int = libc::SOCK_STREAM;
pub(crate) const SOCK_DGRAM: libc::c_int = libc::SOCK_DGRAM;
pub(crate) const SOCK_RAW: libc::c_int = libc::SOCK_RAW;
pub(crate) const SOCK_SEQPACKET: libc::c_int = libc::SOCK_SEQPACKET;
pub(crate) const IPPROTO_IP: libc::c_int = libc::IPPROTO_IP;
pub(crate) const IPPROTO_TCP: libc::c_int = libc::IPPROTO_TCP;
pub(crate) const IPPROTO_UDP: libc::c_int = libc::IPPROTO_UDP;

pub(crate) const INVALID_SOCKET: RawSocket = -1;

//==================================================================================================
// Shutdown Constants
//==================================================================================================

pub(crate) const SHUT_RD: libc::c_int = libc::SHUT_RD;
pub(crate) const SHUT_WR: libc::c_int = libc::SHUT_WR;
pub(crate) const SHUT_RDWR: libc::c_int = libc::SHUT_RDWR;

//==================================================================================================
// Socketpair Support
//==================================================================================================

/// Whether `socketpair()` is supported on this platform.
pub(crate) const SOCKETPAIR_SUPPORTED: bool = true;

//==================================================================================================
// Handle Conversion
//==================================================================================================

/// Convert a platform `RawSocket` to the `i32` used in the Nanvix guest ABI.
#[inline]
pub(crate) fn raw_to_i32(s: RawSocket) -> i32 {
    s
}

/// Convert a Nanvix guest `i32` socket fd to the platform `RawSocket`.
#[inline]
pub(crate) fn i32_to_raw(fd: i32) -> RawSocket {
    fd as RawSocket
}

//==================================================================================================
// Error Handling
//==================================================================================================

/// Returns the last socket error code.
#[inline]
pub(crate) fn last_socket_error() -> i32 {
    unsafe { *libc::__errno_location() }
}

/// Check if a socket operation was interrupted by a signal (`EINTR`).
#[inline]
pub(crate) fn is_interrupted(errno: i32) -> bool {
    errno == libc::EINTR
}

/// Check whether a socket operation result indicates failure.
#[inline]
pub(crate) fn socket_failed(result: RawSocket) -> bool {
    result == INVALID_SOCKET
}

/// Maps a host (Linux) `errno` to the Nanvix `errno` numbering consumed by `ErrorCode::try_from`.
///
/// Nanvix uses a newlib-style `errno` layout (see `sysapi::errno`) whose socket/network range does
/// not match Linux's `asm-generic` layout. For example `EINPROGRESS` is 115 on Linux but 119 on
/// Nanvix, and `EISCONN` is 106 versus 127. Feeding a raw host `errno` into `ErrorCode::try_from`
/// (which matches Nanvix constants) therefore mis-maps these codes: a non-blocking `connect()`
/// returning `EINPROGRESS` would be classified as a fatal error instead of a "connection pending"
/// condition the epoll reactor should park on.
///
/// `net-backend` is the only layer that converts a host socket-syscall `errno` into a Nanvix
/// `ErrorCode`, and it only ever surfaces socket/network `errno`s, so translating that family here
/// is sufficient. The generic low `errno`s (`EBADF`, `EINVAL`, `EAGAIN`, `EINTR`, ...) are `< 35`
/// and identical between Linux and Nanvix, so they fall through the identity arm unchanged.
#[inline]
pub(crate) fn normalize_errno(errno: i32) -> i32 {
    // Only the socket/network `errno`s whose Nanvix numbering diverges from Linux need remapping;
    // everything else already matches and is passed through untouched.
    match errno {
        libc::EADDRINUSE => ::sysapi::errno::EADDRINUSE,
        libc::EADDRNOTAVAIL => ::sysapi::errno::EADDRNOTAVAIL,
        libc::EAFNOSUPPORT => ::sysapi::errno::EAFNOSUPPORT,
        libc::EALREADY => ::sysapi::errno::EALREADY,
        libc::ECONNABORTED => ::sysapi::errno::ECONNABORTED,
        libc::EDESTADDRREQ => ::sysapi::errno::EDESTADDRREQ,
        libc::EHOSTDOWN => ::sysapi::errno::EHOSTDOWN,
        libc::EHOSTUNREACH => ::sysapi::errno::EHOSTUNREACH,
        libc::EINPROGRESS => ::sysapi::errno::EINPROGRESS,
        libc::EISCONN => ::sysapi::errno::EISCONN,
        libc::EMSGSIZE => ::sysapi::errno::EMSGSIZE,
        libc::ENETDOWN => ::sysapi::errno::ENETDOWN,
        libc::ENETRESET => ::sysapi::errno::ENETRESET,
        libc::ENETUNREACH => ::sysapi::errno::ENETUNREACH,
        libc::ENOPROTOOPT => ::sysapi::errno::ENOPROTOOPT,
        libc::ENOTCONN => ::sysapi::errno::ENOTCONN,
        libc::ENOTSOCK => ::sysapi::errno::ENOTSOCK,
        libc::EPROTONOSUPPORT => ::sysapi::errno::EPROTONOSUPPORT,
        libc::EPROTOTYPE => ::sysapi::errno::EPROTOTYPE,
        libc::ESHUTDOWN => ::sysapi::errno::ESHUTDOWN,
        libc::ESOCKTNOSUPPORT => ::sysapi::errno::ESOCKTNOSUPPORT,
        libc::ETIMEDOUT => ::sysapi::errno::ETIMEDOUT,
        libc::ETOOMANYREFS => ::sysapi::errno::ETOOMANYREFS,
        libc::EUSERS => ::sysapi::errno::EUSERS,
        other => other,
    }
}

/// Maps a host `connect()` errno to the Nanvix errno numbering consumed by
/// `ErrorCode::try_from`.
#[inline]
pub(crate) fn normalize_connect_errno(errno: i32) -> i32 {
    // Unix does not need connect-specific errno handling, but Windows does because
    // `WSAEWOULDBLOCK` means pending connect on that path and generic would-block elsewhere.
    normalize_errno(errno)
}

//==================================================================================================
// Socket Close
//==================================================================================================

/// Close a socket handle.
#[inline]
pub(crate) unsafe fn close_socket(fd: RawSocket) -> libc::c_int {
    libc::close(fd)
}

//==================================================================================================
// Address Helpers
//==================================================================================================

/// Platform-specific `sockaddr.sa_data` conversion from `[u8; 14]`.
#[inline]
pub(crate) fn sa_data_from_u8(data: [u8; 14]) -> [i8; 14] {
    unsafe { core::mem::transmute::<[u8; 14], [i8; 14]>(data) }
}

/// Platform-specific `sockaddr.sa_data` conversion to `[u8; 14]`.
#[inline]
pub(crate) fn sa_data_to_u8(data: [i8; 14]) -> [u8; 14] {
    unsafe { core::mem::transmute::<[i8; 14], [u8; 14]>(data) }
}

/// Platform-specific `sa_family_t` for sockaddr construction.
#[inline]
pub(crate) fn to_sa_family(fam: SaFamilyT) -> libc::sa_family_t {
    fam
}

//==================================================================================================
// Winsock Initialization (no-op on Unix)
//==================================================================================================

/// On Unix, no initialization is needed.
pub(crate) fn init() -> Result<(), crate::error::NetError> {
    Ok(())
}

//==================================================================================================
// Raw Socket Operations
//==================================================================================================

/// Maximum byte count that can be passed to a single `send()`/`recv()` call.
/// On Unix this is `usize::MAX` (the kernel will clamp internally).
pub(crate) const MAX_IO_LEN: usize = usize::MAX;

/// Raw send operation.
#[inline]
pub(crate) unsafe fn raw_send(
    fd: RawSocket,
    buf: *const u8,
    count: usize,
    flags: libc::c_int,
) -> isize {
    libc::send(fd, buf as *const libc::c_void, count as _, flags) as isize
}

/// Raw recv operation.
#[inline]
pub(crate) unsafe fn raw_recv(
    fd: RawSocket,
    buf: *mut u8,
    count: usize,
    flags: libc::c_int,
) -> isize {
    libc::recv(fd, buf as *mut libc::c_void, count as _, flags) as isize
}

/// Raw sendto operation.
#[inline]
pub(crate) unsafe fn raw_sendto(
    fd: RawSocket,
    buf: *const u8,
    count: usize,
    flags: libc::c_int,
    addr: *const libc::sockaddr,
    addrlen: SocklenT,
) -> isize {
    libc::sendto(fd, buf as *const libc::c_void, count as _, flags, addr, addrlen) as isize
}

/// Raw recvfrom operation.
#[inline]
pub(crate) unsafe fn raw_recvfrom(
    fd: RawSocket,
    buf: *mut u8,
    count: usize,
    flags: libc::c_int,
    addr: *mut libc::sockaddr,
    addrlen: *mut SocklenT,
) -> isize {
    libc::recvfrom(fd, buf as *mut libc::c_void, count as _, flags, addr, addrlen) as isize
}

/// Raw shutdown operation.
#[inline]
pub(crate) unsafe fn raw_shutdown(fd: RawSocket, how: libc::c_int) -> libc::c_int {
    libc::shutdown(fd, how)
}

/// Raw socketpair operation.
#[inline]
pub(crate) unsafe fn raw_socketpair(
    domain: libc::c_int,
    typ: libc::c_int,
    protocol: libc::c_int,
    sv: &mut [libc::c_int; 2],
) -> libc::c_int {
    libc::socketpair(domain, typ, protocol, sv.as_mut_ptr())
}

/// Raw operation to enable or disable non-blocking mode on a socket.
///
/// Uses `fcntl(F_GETFL)` followed by `fcntl(F_SETFL)` to toggle `O_NONBLOCK`. Returns 0 on success
/// and -1 on failure, with the platform errno set.
///
/// # Safety
///
/// `fd` must be a file descriptor that is valid for `fcntl()` on this process. The caller is
/// responsible for ensuring that concurrent users of the descriptor tolerate status-flag changes.
#[inline]
pub(crate) unsafe fn raw_set_nonblocking(fd: RawSocket, nonblocking: bool) -> libc::c_int {
    let flags: libc::c_int = libc::fcntl(fd, libc::F_GETFL, 0);
    if flags < 0 {
        return -1;
    }
    let new_flags: libc::c_int = if nonblocking {
        flags | libc::O_NONBLOCK
    } else {
        flags & !libc::O_NONBLOCK
    };
    libc::fcntl(fd, libc::F_SETFL, new_flags)
}

//==================================================================================================
// Message Flags
//==================================================================================================

use ::sysapi::netinet_in::message_flags::{
    MSG_EOR,
    MSG_NOSIGNAL,
    MSG_OOB,
    MSG_PEEK,
    MSG_WAITALL,
};

/// Platform-specific message flag mapping.
/// Returns the array of (nanvix_flag, platform_flag) pairs and a set of flags to silently strip.
pub(crate) fn message_flag_mappings() -> (&'static [(i32, libc::c_int)], &'static [i32]) {
    static MAPPINGS: [(i32, libc::c_int); 5] = [
        (MSG_PEEK, libc::MSG_PEEK),
        (MSG_OOB, libc::MSG_OOB),
        (MSG_WAITALL, libc::MSG_WAITALL),
        (MSG_EOR, libc::MSG_EOR),
        (MSG_NOSIGNAL, libc::MSG_NOSIGNAL),
    ];
    // No flags to silently strip on Unix.
    static STRIP: [i32; 0] = [];
    (&MAPPINGS, &STRIP)
}

//==================================================================================================
// Shutdown Mapping
//==================================================================================================

use ::syscall::sys::socket::Shutdown;

/// Map a Nanvix Shutdown reason to the platform constant.
pub(crate) fn shutdown_to_platform(how: Shutdown) -> libc::c_int {
    match how {
        Shutdown::Read => SHUT_RD,
        Shutdown::Write => SHUT_WR,
        Shutdown::ReadWrite => SHUT_RDWR,
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod test {
    use super::*;

    // ---- sa_data roundtrip ----------------------------------------------------------------------

    /// Tests that `sa_data_from_u8` / `sa_data_to_u8` roundtrips zeros.
    #[test]
    fn sa_data_roundtrip_zeros() {
        let input: [u8; 14] = [0u8; 14];
        let result: [u8; 14] = sa_data_to_u8(sa_data_from_u8(input));
        assert_eq!(result, input, "zero roundtrip should be identity");
    }

    /// Tests that `sa_data_from_u8` / `sa_data_to_u8` roundtrips non-zero data.
    #[test]
    fn sa_data_roundtrip_nonzero() {
        let input: [u8; 14] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
        let result: [u8; 14] = sa_data_to_u8(sa_data_from_u8(input));
        assert_eq!(result, input, "non-zero roundtrip should be identity");
    }

    // ---- normalize_errno ------------------------------------------------------------------------

    /// Tests that `normalize_errno` passes through `errno`s that already match Nanvix numbering.
    #[test]
    fn normalize_errno_passthrough() {
        // Generic low `errno`s are identical between Linux and Nanvix.
        assert_eq!(normalize_errno(libc::EINTR), libc::EINTR, "EINTR should pass through");
        assert_eq!(normalize_errno(libc::EINVAL), libc::EINVAL, "EINVAL should pass through");
        assert_eq!(normalize_errno(libc::EBADF), libc::EBADF, "EBADF should pass through");
        assert_eq!(normalize_errno(libc::EAGAIN), libc::EAGAIN, "EAGAIN should pass through");
        // A handful of socket `errno`s also happen to match and must not be remapped.
        assert_eq!(
            normalize_errno(libc::ECONNREFUSED),
            libc::ECONNREFUSED,
            "ECONNREFUSED should pass through"
        );
        assert_eq!(normalize_errno(42), 42, "arbitrary value should pass through");
    }

    /// Tests that `normalize_errno` translates diverging socket `errno`s to Nanvix numbering, and
    /// that the result round-trips into the `ErrorCode` variants the epoll reactor branches on.
    #[test]
    fn normalize_errno_translates_socket_family() {
        use ::sys::error::ErrorCode;

        // Raw Linux values differ from Nanvix values for these codes.
        assert_eq!(normalize_errno(libc::EINPROGRESS), ::sysapi::errno::EINPROGRESS);
        assert_eq!(normalize_errno(libc::EALREADY), ::sysapi::errno::EALREADY);
        assert_eq!(normalize_errno(libc::EISCONN), ::sysapi::errno::EISCONN);

        // Round-trip into the `ErrorCode`s the networkd reactor's connect path depends on:
        // without the translation these would map to unrelated variants and connect() would fail
        // spuriously.
        assert_eq!(
            ErrorCode::try_from(normalize_errno(libc::EINPROGRESS)).unwrap(),
            ErrorCode::OperationInProgress,
            "EINPROGRESS must classify as connection-pending"
        );
        assert_eq!(
            ErrorCode::try_from(normalize_errno(libc::EALREADY)).unwrap(),
            ErrorCode::OperationAlreadyInProgress,
            "EALREADY must classify as already-in-progress"
        );
        assert_eq!(
            ErrorCode::try_from(normalize_errno(libc::EISCONN)).unwrap(),
            ErrorCode::TransportEndpointConnected,
            "EISCONN must classify as already-connected"
        );
    }

    /// Tests that `normalize_connect_errno` preserves the generic Unix errno mapping.
    #[test]
    fn normalize_connect_errno_uses_socket_family_mapping() {
        use ::sys::error::ErrorCode;

        assert_eq!(
            ErrorCode::try_from(normalize_connect_errno(libc::EINPROGRESS)).unwrap(),
            ErrorCode::OperationInProgress,
            "EINPROGRESS must classify as connection-pending"
        );
        assert_eq!(
            ErrorCode::try_from(normalize_connect_errno(libc::EALREADY)).unwrap(),
            ErrorCode::OperationAlreadyInProgress,
            "EALREADY must classify as already-in-progress"
        );
    }

    // ---- Boolean helpers ------------------------------------------------------------------------

    /// Tests that `is_interrupted` returns true for `EINTR`.
    #[test]
    fn is_interrupted_true_for_eintr() {
        assert!(is_interrupted(libc::EINTR), "EINTR should be detected as interrupted");
    }

    /// Tests that `is_interrupted` returns false for zero.
    #[test]
    fn is_interrupted_false_for_zero() {
        assert!(!is_interrupted(0), "zero should not be interrupted");
    }

    /// Tests that `socket_failed` returns true for `INVALID_SOCKET` (-1).
    #[test]
    fn socket_failed_true_for_invalid() {
        assert!(socket_failed(INVALID_SOCKET), "INVALID_SOCKET should indicate failure");
    }

    /// Tests that `socket_failed` returns false for a valid fd.
    #[test]
    fn socket_failed_false_for_valid() {
        assert!(!socket_failed(3), "valid fd should not indicate failure");
    }

    // ---- Handle conversion ----------------------------------------------------------------------

    /// Tests that `raw_to_i32` / `i32_to_raw` roundtrips on Unix.
    #[test]
    fn handle_conversion_roundtrip() {
        let original: i32 = 42;
        let raw: RawSocket = i32_to_raw(original);
        let back: i32 = raw_to_i32(raw);
        assert_eq!(back, original, "handle roundtrip should be identity");
    }
}
