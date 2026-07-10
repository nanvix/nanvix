// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Windows Platform Implementation
//==================================================================================================

//! Windows-specific networking primitives using Winsock2.

//==================================================================================================
// Type Aliases
//==================================================================================================

pub(crate) type SaFamilyT = libc::c_ushort;
pub(crate) type SocklenT = libc::c_int;
pub(crate) type RawSocket = libc::SOCKET;

//==================================================================================================
// Socket Constants
//==================================================================================================

pub(crate) const AF_INET: libc::c_int = 2;
pub(crate) const AF_INET6: libc::c_int = 23;
pub(crate) const AF_UNIX: libc::c_int = 1;
pub(crate) const SOCK_STREAM: libc::c_int = 1;
pub(crate) const SOCK_DGRAM: libc::c_int = 2;
pub(crate) const SOCK_RAW: libc::c_int = 3;
pub(crate) const SOCK_SEQPACKET: libc::c_int = 5;
pub(crate) const IPPROTO_IP: libc::c_int = 0;
pub(crate) const IPPROTO_TCP: libc::c_int = 6;
pub(crate) const IPPROTO_UDP: libc::c_int = 17;

pub(crate) const INVALID_SOCKET: RawSocket = !0usize;

//==================================================================================================
// Shutdown Constants
//==================================================================================================

pub(crate) const SHUT_RD: libc::c_int = 0; // SD_RECEIVE
pub(crate) const SHUT_WR: libc::c_int = 1; // SD_SEND
pub(crate) const SHUT_RDWR: libc::c_int = 2; // SD_BOTH

//==================================================================================================
// Socketpair Support
//==================================================================================================

/// Whether `socketpair()` is supported on this platform.
pub(crate) const SOCKETPAIR_SUPPORTED: bool = false;

//==================================================================================================
// Handle Conversion
//==================================================================================================

/// Convert a platform `RawSocket` to the `i32` used in the Nanvix guest ABI.
///
/// On 64-bit Windows, `SOCKET` is `usize`. Handles returned by Winsock fit in the lower
/// 32 bits in practice, but this function validates the range and panics on overflow to
/// prevent silent truncation.
#[inline]
pub(crate) fn raw_to_i32(s: RawSocket) -> i32 {
    i32::try_from(s).expect("SOCKET handle exceeds i32 range")
}

/// Convert a Nanvix guest `i32` socket fd to the platform `RawSocket`.
///
/// Negative values are sign-extended to `usize`, which maps to `INVALID_SOCKET` on
/// 64-bit Windows. This is intentional: callers pass -1 to test error paths.
#[inline]
pub(crate) fn i32_to_raw(fd: i32) -> RawSocket {
    fd as RawSocket
}

//==================================================================================================
// Error Handling
//==================================================================================================

/// Returns the last socket error code (calls `WSAGetLastError`).
#[inline]
pub(crate) fn last_socket_error() -> i32 {
    unsafe { winsock::WSAGetLastError() }
}

/// Check if a socket operation returned an interrupted error.
#[inline]
pub(crate) fn is_interrupted(errno: i32) -> bool {
    errno == winsock::WSAEINTR
}

/// Check whether a socket operation result indicates failure.
#[inline]
pub(crate) fn socket_failed(result: RawSocket) -> bool {
    result == INVALID_SOCKET
}

//==================================================================================================
// Socket Close
//==================================================================================================

/// Close a socket handle (calls `closesocket`).
#[inline]
pub(crate) unsafe fn close_socket(fd: RawSocket) -> libc::c_int {
    winsock::closesocket(fd)
}

//==================================================================================================
// Address Helpers
//==================================================================================================

/// Platform-specific `sockaddr.sa_data` conversion from `[u8; 14]`.
#[inline]
pub(crate) fn sa_data_from_u8(data: [u8; 14]) -> [libc::c_char; 14] {
    unsafe { core::mem::transmute::<[u8; 14], [libc::c_char; 14]>(data) }
}

/// Platform-specific `sockaddr.sa_data` conversion to `[u8; 14]`.
#[inline]
pub(crate) fn sa_data_to_u8(data: [libc::c_char; 14]) -> [u8; 14] {
    unsafe { core::mem::transmute::<[libc::c_char; 14], [u8; 14]>(data) }
}

/// Platform-specific `sa_family_t` for sockaddr construction.
#[inline]
pub(crate) fn to_sa_family(fam: SaFamilyT) -> libc::c_ushort {
    fam
}

//==================================================================================================
// Winsock Initialization
//==================================================================================================

/// Initializes Winsock. Must be called before any socket operation.
///
/// Returns `Err` if `WSAStartup` fails. The result is cached so that subsequent calls
/// return immediately without re-initializing.
pub(crate) fn init() -> Result<(), crate::error::NetError> {
    use ::std::sync::OnceLock;
    static INIT: OnceLock<Result<(), ::sys::error::ErrorCode>> = OnceLock::new();
    let result: &Result<(), ::sys::error::ErrorCode> = INIT.get_or_init(|| unsafe {
        let mut wsa_data: winsock::WSADATA = core::mem::zeroed();
        let ret: libc::c_int = winsock::WSAStartup(0x0202, &mut wsa_data);
        if ret != 0 {
            ::log::error!("WSAStartup failed with error: {ret}");
            Err(::sys::error::ErrorCode::IoErr)
        } else {
            Ok(())
        }
    });
    result.map_err(crate::error::NetError::Errno)
}

//==================================================================================================
// Raw Socket Operations
//==================================================================================================

/// Maximum byte count that can be passed to a single Winsock `send()`/`recv()` call.
pub(crate) const MAX_IO_LEN: usize = libc::c_int::MAX as usize;

/// Raw send operation (calls Winsock `send`).
///
/// # Safety
///
/// `buf` must point to at least `count` readable bytes. `count` must not exceed
/// `libc::c_int::MAX`; the caller is responsible for validating this.
#[inline]
pub(crate) unsafe fn raw_send(
    fd: RawSocket,
    buf: *const u8,
    count: usize,
    flags: libc::c_int,
) -> isize {
    winsock::send(fd, buf as *const libc::c_char, count as libc::c_int, flags) as isize
}

/// Raw recv operation (calls Winsock `recv`).
///
/// # Safety
///
/// `buf` must point to at least `count` writable bytes. `count` must not exceed
/// `libc::c_int::MAX`; the caller is responsible for validating this.
#[inline]
pub(crate) unsafe fn raw_recv(
    fd: RawSocket,
    buf: *mut u8,
    count: usize,
    flags: libc::c_int,
) -> isize {
    winsock::recv(fd, buf as *mut libc::c_char, count as libc::c_int, flags) as isize
}

/// Raw sendto operation (calls Winsock `sendto`).
///
/// # Safety
///
/// `buf` must point to at least `count` readable bytes. `count` must not exceed
/// `libc::c_int::MAX`; the caller is responsible for validating this. `addr` must point to a
/// valid socket address of `addrlen` bytes.
#[inline]
pub(crate) unsafe fn raw_sendto(
    fd: RawSocket,
    buf: *const u8,
    count: usize,
    flags: libc::c_int,
    addr: *const libc::sockaddr,
    addrlen: SocklenT,
) -> isize {
    winsock::sendto(fd, buf as *const libc::c_char, count as libc::c_int, flags, addr, addrlen)
        as isize
}

/// Raw recvfrom operation (calls Winsock `recvfrom`).
///
/// # Safety
///
/// `buf` must point to at least `count` writable bytes. `count` must not exceed
/// `libc::c_int::MAX`; the caller is responsible for validating this. `addr` and `addrlen` must
/// point to a valid socket address buffer and its length.
#[inline]
pub(crate) unsafe fn raw_recvfrom(
    fd: RawSocket,
    buf: *mut u8,
    count: usize,
    flags: libc::c_int,
    addr: *mut libc::sockaddr,
    addrlen: *mut SocklenT,
) -> isize {
    winsock::recvfrom(fd, buf as *mut libc::c_char, count as libc::c_int, flags, addr, addrlen)
        as isize
}

/// Raw shutdown operation (calls Winsock `shutdown`).
#[inline]
pub(crate) unsafe fn raw_shutdown(fd: RawSocket, how: libc::c_int) -> libc::c_int {
    winsock::shutdown(fd, how)
}

/// Raw socketpair operation — not supported on Windows.
/// Always returns -1. Caller must check `SOCKETPAIR_SUPPORTED` first.
#[inline]
pub(crate) unsafe fn raw_socketpair(
    _domain: libc::c_int,
    _typ: libc::c_int,
    _protocol: libc::c_int,
    _sv: &mut [libc::c_int; 2],
) -> libc::c_int {
    -1
}

/// Raw operation to enable or disable non-blocking mode on a socket.
///
/// Uses `ioctlsocket(FIONBIO)`. Returns 0 on success and `SOCKET_ERROR` (-1) on failure, with the
/// Winsock error retrievable via `WSAGetLastError`.
///
/// # Safety
///
/// `fd` must be a socket handle that is valid for `ioctlsocket()` on this process. The caller is
/// responsible for ensuring that concurrent users of the socket tolerate mode changes.
#[inline]
pub(crate) unsafe fn raw_set_nonblocking(fd: RawSocket, nonblocking: bool) -> libc::c_int {
    let mut mode: libc::c_ulong = if nonblocking { 1 } else { 0 };
    winsock::ioctlsocket(fd, winsock::FIONBIO, &mut mode as *mut libc::c_ulong)
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
    static MAPPINGS: [(i32, libc::c_int); 3] = [
        (MSG_PEEK, winsock::MSG_PEEK),
        (MSG_OOB, winsock::MSG_OOB),
        (MSG_WAITALL, winsock::MSG_WAITALL),
    ];
    // MSG_NOSIGNAL and MSG_EOR don't exist on Windows; strip them silently.
    static STRIP: [i32; 2] = [MSG_NOSIGNAL, MSG_EOR];
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
// Winsock FFI Bindings
//==================================================================================================

pub(crate) mod winsock {
    //! Minimal Winsock2 FFI bindings for functions missing from the `libc` crate on Windows.

    use libc::{
        c_char,
        c_int,
        c_long,
        c_ulong,
        sockaddr,
        SOCKET,
    };

    // Winsock error codes
    pub const WSAEINTR: i32 = 10004;

    // Message flags
    pub const MSG_PEEK: c_int = 0x2;
    pub const MSG_OOB: c_int = 0x1;
    pub const MSG_WAITALL: c_int = 0x8;

    // ioctlsocket command to enable/disable non-blocking mode (FIONBIO).
    pub const FIONBIO: c_long = 0x8004667Eu32 as c_long;

    /// WSADATA structure for WSAStartup.
    #[repr(C)]
    #[allow(clippy::upper_case_acronyms)]
    pub struct WSADATA {
        pub w_version: u16,
        pub w_high_version: u16,
        #[cfg(target_pointer_width = "64")]
        pub sz_description: [u8; 257],
        #[cfg(target_pointer_width = "64")]
        pub sz_system_status: [u8; 129],
        #[cfg(target_pointer_width = "64")]
        pub i_max_sockets: u16,
        #[cfg(target_pointer_width = "64")]
        pub i_max_udp_dg: u16,
        #[cfg(target_pointer_width = "64")]
        pub lp_vendor_info: *mut u8,
        #[cfg(target_pointer_width = "32")]
        pub sz_description: [u8; 257],
        #[cfg(target_pointer_width = "32")]
        pub sz_system_status: [u8; 129],
        #[cfg(target_pointer_width = "32")]
        pub i_max_sockets: u16,
        #[cfg(target_pointer_width = "32")]
        pub i_max_udp_dg: u16,
        #[cfg(target_pointer_width = "32")]
        pub lp_vendor_info: *mut u8,
    }

    #[link(name = "ws2_32")]
    extern "system" {
        pub fn WSAStartup(wVersionRequested: u16, lpWSAData: *mut WSADATA) -> c_int;
        #[allow(dead_code)]
        pub fn WSACleanup() -> c_int;
        pub fn WSAGetLastError() -> c_int;
        pub fn closesocket(s: SOCKET) -> c_int;
        pub fn ioctlsocket(s: SOCKET, cmd: c_long, argp: *mut c_ulong) -> c_int;
        pub fn shutdown(s: SOCKET, how: c_int) -> c_int;
        pub fn send(s: SOCKET, buf: *const c_char, len: c_int, flags: c_int) -> c_int;
        pub fn recv(s: SOCKET, buf: *mut c_char, len: c_int, flags: c_int) -> c_int;
        pub fn sendto(
            s: SOCKET,
            buf: *const c_char,
            len: c_int,
            flags: c_int,
            to: *const sockaddr,
            tolen: c_int,
        ) -> c_int;
        pub fn recvfrom(
            s: SOCKET,
            buf: *mut c_char,
            len: c_int,
            flags: c_int,
            from: *mut sockaddr,
            fromlen: *mut c_int,
        ) -> c_int;
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

    // ---- Boolean helpers ------------------------------------------------------------------------

    /// Tests that `is_interrupted` returns true for `WSAEINTR`.
    #[test]
    fn is_interrupted_true_for_wsaeintr() {
        assert!(is_interrupted(winsock::WSAEINTR), "WSAEINTR should be detected as interrupted");
    }

    /// Tests that `is_interrupted` returns false for zero.
    #[test]
    fn is_interrupted_false_for_zero() {
        assert!(!is_interrupted(0), "zero should not be interrupted");
    }

    /// Tests that `socket_failed` returns true for `INVALID_SOCKET`.
    #[test]
    fn socket_failed_true_for_invalid() {
        assert!(socket_failed(INVALID_SOCKET), "INVALID_SOCKET should indicate failure");
    }

    /// Tests that `socket_failed` returns false for a valid handle.
    #[test]
    fn socket_failed_false_for_valid() {
        assert!(!socket_failed(3), "valid handle should not indicate failure");
    }

    // ---- Handle conversion ----------------------------------------------------------------------

    /// Tests that `raw_to_i32` / `i32_to_raw` roundtrips on Windows.
    #[test]
    fn handle_conversion_roundtrip() {
        let original: i32 = 42;
        let raw: RawSocket = i32_to_raw(original);
        let back: i32 = raw_to_i32(raw);
        assert_eq!(back, original, "handle roundtrip should be identity");
    }
}
