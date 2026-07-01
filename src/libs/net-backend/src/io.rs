// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::NetError,
    platform::{
        i32_to_raw,
        is_interrupted,
        last_socket_error,
        normalize_errno,
        raw_recv,
        raw_recvfrom,
        raw_send,
        raw_sendto,
        sa_data_to_u8,
        SocklenT,
        MAX_IO_LEN,
    },
    types::{
        LibcMessageFlags,
        LibcSocketAddress,
    },
    NetBackend,
};
use ::core::mem;
use ::log::{
    debug,
    error,
};
use ::sys::error::ErrorCode;
use ::sysapi::sys_socket::sockaddr;

//==================================================================================================
// I/O Operations
//==================================================================================================

impl NetBackend {
    /// Sends data on a connected socket.
    ///
    /// Returns the number of bytes sent.
    pub fn send(
        &self,
        sockfd: i32,
        buf: &[u8],
        count: usize,
        flags: i32,
    ) -> Result<isize, NetError> {
        // Validate that count does not exceed buffer length to prevent out-of-bounds reads.
        if count > buf.len() {
            error!("send(): count ({count}) exceeds buffer length ({})", buf.len());
            return Err(NetError::Errno(ErrorCode::InvalidArgument));
        }

        // On Windows, Winsock send() takes a c_int length; reject oversized counts.
        // On Unix MAX_IO_LEN is usize::MAX so the comparison is trivially false;
        // allow the lint so the check stays for Windows.
        #[allow(clippy::absurd_extreme_comparisons)]
        if count > MAX_IO_LEN {
            error!("send(): count ({count}) exceeds platform maximum ({MAX_IO_LEN})");
            return Err(NetError::Errno(ErrorCode::InvalidArgument));
        }

        let flags: LibcMessageFlags =
            LibcMessageFlags::try_from_nanvix(flags).map_err(|e| NetError::Errno(e.code))?;

        debug!("libc::send(): sockfd={sockfd:?}, count={count:?}, flags={:?}", flags.inner());

        let raw = i32_to_raw(sockfd);
        let result = unsafe { raw_send(raw, buf.as_ptr(), count, flags.inner()) };

        if result >= 0 {
            debug!("libc::send(): count={result:?}");
            Ok(result)
        } else {
            let errno: i32 = last_socket_error();
            if is_interrupted(errno) {
                return Err(NetError::Interrupted);
            }
            error!("libc::send(): failed with errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(normalize_errno(errno)).unwrap_or(ErrorCode::ValueOutOfRange);
            Err(NetError::Errno(error))
        }
    }

    /// Receives data from a connected socket.
    ///
    /// Returns the number of bytes received.
    pub fn recv(
        &self,
        sockfd: i32,
        buf: &mut [u8],
        count: usize,
        flags: i32,
    ) -> Result<isize, NetError> {
        // Validate that count does not exceed buffer length to prevent out-of-bounds writes.
        if count > buf.len() {
            error!("recv(): count ({count}) exceeds buffer length ({})", buf.len());
            return Err(NetError::Errno(ErrorCode::InvalidArgument));
        }

        // On Windows, Winsock recv() takes a c_int length; reject oversized counts.
        // On Unix MAX_IO_LEN is usize::MAX so the comparison is trivially false;
        // allow the lint so the check stays for Windows.
        #[allow(clippy::absurd_extreme_comparisons)]
        if count > MAX_IO_LEN {
            error!("recv(): count ({count}) exceeds platform maximum ({MAX_IO_LEN})");
            return Err(NetError::Errno(ErrorCode::InvalidArgument));
        }

        let flags: LibcMessageFlags =
            LibcMessageFlags::try_from_nanvix(flags).map_err(|e| NetError::Errno(e.code))?;

        debug!("libc::recv(): sockfd={sockfd:?}, len={count:?}, flags={:?}", flags.inner());

        let raw = i32_to_raw(sockfd);
        let result = unsafe { raw_recv(raw, buf.as_mut_ptr(), count, flags.inner()) };

        if result >= 0 {
            debug!("libc::recv(): count={result:?}");
            Ok(result)
        } else {
            let errno: i32 = last_socket_error();
            if is_interrupted(errno) {
                return Err(NetError::Interrupted);
            }
            error!("libc::recv(): failed with errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(normalize_errno(errno)).unwrap_or(ErrorCode::ValueOutOfRange);
            Err(NetError::Errno(error))
        }
    }

    /// Sends a datagram on a socket to an explicit destination address.
    ///
    /// Returns the number of bytes sent.
    pub fn sendto(
        &self,
        sockfd: i32,
        buf: &[u8],
        count: usize,
        flags: i32,
        addr: &sockaddr,
    ) -> Result<isize, NetError> {
        // Validate that count does not exceed buffer length to prevent out-of-bounds reads.
        if count > buf.len() {
            error!("sendto(): count ({count}) exceeds buffer length ({})", buf.len());
            return Err(NetError::Errno(ErrorCode::InvalidArgument));
        }

        // On Windows, Winsock sendto() takes a c_int length; reject oversized counts.
        // On Unix MAX_IO_LEN is usize::MAX so the comparison is trivially false;
        // allow the lint so the check stays for Windows.
        #[allow(clippy::absurd_extreme_comparisons)]
        if count > MAX_IO_LEN {
            error!("sendto(): count ({count}) exceeds platform maximum ({MAX_IO_LEN})");
            return Err(NetError::Errno(ErrorCode::InvalidArgument));
        }

        let flags: LibcMessageFlags =
            LibcMessageFlags::try_from_nanvix(flags).map_err(|e| NetError::Errno(e.code))?;

        let sockaddr: LibcSocketAddress =
            LibcSocketAddress::try_from(*addr).map_err(|e| NetError::Errno(e.code))?;
        let inner: libc::sockaddr = sockaddr.inner();
        let socklen: SocklenT = mem::size_of_val(&inner) as SocklenT;

        debug!(
            "libc::sendto(): sockfd={sockfd:?}, count={count:?}, flags={:?}, \
             sockaddr.sa_family={:?}",
            flags.inner(),
            inner.sa_family,
        );

        let raw = i32_to_raw(sockfd);
        let result = unsafe {
            raw_sendto(
                raw,
                buf.as_ptr(),
                count,
                flags.inner(),
                &inner as *const libc::sockaddr,
                socklen,
            )
        };

        if result >= 0 {
            debug!("libc::sendto(): count={result:?}");
            Ok(result)
        } else {
            let errno: i32 = last_socket_error();
            if is_interrupted(errno) {
                return Err(NetError::Interrupted);
            }
            error!("libc::sendto(): failed with errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(normalize_errno(errno)).unwrap_or(ErrorCode::ValueOutOfRange);
            Err(NetError::Errno(error))
        }
    }

    /// Receives a datagram on a socket and reports the source address.
    ///
    /// Returns the number of bytes received and the source address.
    pub fn recvfrom(
        &self,
        sockfd: i32,
        buf: &mut [u8],
        count: usize,
        flags: i32,
    ) -> Result<(isize, sockaddr), NetError> {
        // Validate that count does not exceed buffer length to prevent out-of-bounds writes.
        if count > buf.len() {
            error!("recvfrom(): count ({count}) exceeds buffer length ({})", buf.len());
            return Err(NetError::Errno(ErrorCode::InvalidArgument));
        }

        // On Windows, Winsock recvfrom() takes a c_int length; reject oversized counts.
        // On Unix MAX_IO_LEN is usize::MAX so the comparison is trivially false;
        // allow the lint so the check stays for Windows.
        #[allow(clippy::absurd_extreme_comparisons)]
        if count > MAX_IO_LEN {
            error!("recvfrom(): count ({count}) exceeds platform maximum ({MAX_IO_LEN})");
            return Err(NetError::Errno(ErrorCode::InvalidArgument));
        }

        let flags: LibcMessageFlags =
            LibcMessageFlags::try_from_nanvix(flags).map_err(|e| NetError::Errno(e.code))?;

        debug!("libc::recvfrom(): sockfd={sockfd:?}, len={count:?}, flags={:?}", flags.inner());

        let mut address: libc::sockaddr = unsafe { mem::zeroed() };
        let mut address_len: SocklenT = mem::size_of::<libc::sockaddr>() as SocklenT;

        let raw = i32_to_raw(sockfd);
        let result = unsafe {
            raw_recvfrom(
                raw,
                buf.as_mut_ptr(),
                count,
                flags.inner(),
                &mut address as *mut libc::sockaddr,
                &mut address_len as *mut SocklenT,
            )
        };

        if result >= 0 {
            debug!("libc::recvfrom(): count={result:?}");
            let addr: sockaddr = sockaddr {
                sa_len: address_len as u8,
                sa_family: address.sa_family as u8,
                sa_data: sa_data_to_u8(address.sa_data),
            };
            Ok((result, addr))
        } else {
            let errno: i32 = last_socket_error();
            if is_interrupted(errno) {
                return Err(NetError::Interrupted);
            }
            error!("libc::recvfrom(): failed with errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(normalize_errno(errno)).unwrap_or(ErrorCode::ValueOutOfRange);
            Err(NetError::Errno(error))
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod test {
    use super::*;
    use crate::error::NetError;

    /// Tests that `send()` rejects a count that exceeds the buffer length.
    #[test]
    fn send_count_exceeds_buffer() {
        let backend: NetBackend =
            NetBackend::new().expect("platform initialization should succeed");
        let buf: [u8; 4] = [0; 4];
        let result: Result<isize, NetError> = backend.send(0, &buf, 8, 0);
        assert!(result.is_err(), "send with count > buf.len() should fail");
        match result {
            Err(NetError::Errno(code)) => {
                assert_eq!(code, ErrorCode::InvalidArgument, "error should be InvalidArgument");
            },
            other => panic!("expected Errno(InvalidArgument), got {other:?}"),
        }
    }

    /// Tests that `recv()` rejects a count that exceeds the buffer length.
    #[test]
    fn recv_count_exceeds_buffer() {
        let backend: NetBackend =
            NetBackend::new().expect("platform initialization should succeed");
        let mut buf: [u8; 4] = [0; 4];
        let result: Result<isize, NetError> = backend.recv(0, &mut buf, 8, 0);
        assert!(result.is_err(), "recv with count > buf.len() should fail");
        match result {
            Err(NetError::Errno(code)) => {
                assert_eq!(code, ErrorCode::InvalidArgument, "error should be InvalidArgument");
            },
            other => panic!("expected Errno(InvalidArgument), got {other:?}"),
        }
    }
}
