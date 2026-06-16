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
        sa_data_to_u8,
        SocklenT,
    },
    NetBackend,
};
use ::log::{
    debug,
    error,
};
use ::sys::error::ErrorCode;
use ::sysapi::sys_socket::sockaddr;

//==================================================================================================
// Query Operations
//==================================================================================================

impl NetBackend {
    /// Gets the address of the peer connected to a socket.
    pub fn getpeername(&self, sockfd: i32) -> Result<sockaddr, NetError> {
        let mut address: libc::sockaddr = unsafe { core::mem::zeroed() };
        let mut address_len: SocklenT = core::mem::size_of::<libc::sockaddr>() as SocklenT;

        debug!("libc::getpeername(): sockfd={sockfd:?}");

        let raw = i32_to_raw(sockfd);
        match unsafe { libc::getpeername(raw, &mut address, &mut address_len) } {
            -1 => {
                let errno: i32 = last_socket_error();
                if is_interrupted(errno) {
                    return Err(NetError::Interrupted);
                }
                error!("libc::getpeername(): failed with errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(normalize_errno(errno))
                    .unwrap_or(ErrorCode::ValueOutOfRange);
                Err(NetError::Errno(error))
            },
            _ => {
                let addr: sockaddr = sockaddr {
                    sa_len: address_len as u8,
                    sa_family: address.sa_family as u8,
                    sa_data: sa_data_to_u8(address.sa_data),
                };
                Ok(addr)
            },
        }
    }

    /// Gets the local address of a socket.
    pub fn getsockname(&self, sockfd: i32) -> Result<sockaddr, NetError> {
        let mut address: libc::sockaddr = unsafe { core::mem::zeroed() };
        let mut address_len: SocklenT = core::mem::size_of::<libc::sockaddr>() as SocklenT;

        debug!("libc::getsockname(): sockfd={sockfd:?}");

        let raw = i32_to_raw(sockfd);
        match unsafe { libc::getsockname(raw, &mut address, &mut address_len) } {
            -1 => {
                let errno: i32 = last_socket_error();
                if is_interrupted(errno) {
                    return Err(NetError::Interrupted);
                }
                error!("libc::getsockname(): failed with errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(normalize_errno(errno))
                    .unwrap_or(ErrorCode::ValueOutOfRange);
                Err(NetError::Errno(error))
            },
            _ => {
                let addr: sockaddr = sockaddr {
                    sa_len: address_len as u8,
                    sa_family: address.sa_family as u8,
                    sa_data: sa_data_to_u8(address.sa_data),
                };
                Ok(addr)
            },
        }
    }
}
