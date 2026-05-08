// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::NetError,
    NetBackend,
};
use ::log::{
    debug,
    error,
    warn,
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
        let mut address_len: libc::socklen_t =
            core::mem::size_of::<libc::sockaddr>() as libc::socklen_t;

        debug!("libc::getpeername(): sockfd={sockfd:?}");

        match unsafe { libc::getpeername(sockfd, &mut address, &mut address_len) } {
            -1 => {
                let errno: libc::c_int = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    return Err(NetError::Interrupted);
                }
                error!("libc::getpeername(): failed with errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno).unwrap_or_else(|_| {
                    warn!("unknown errno value {errno:?}, mapping to ValueOutOfRange");
                    ErrorCode::ValueOutOfRange
                });
                Err(NetError::Errno(error))
            },
            _ => {
                let mut sa_data: [u8; 14] = [0u8; 14];
                for (i, &b) in address.sa_data.iter().enumerate() {
                    sa_data[i] = b as u8;
                }
                let addr: sockaddr = sockaddr {
                    sa_len: address_len as u8,
                    sa_family: address.sa_family as u8,
                    sa_data,
                };
                Ok(addr)
            },
        }
    }

    /// Gets the local address of a socket.
    pub fn getsockname(&self, sockfd: i32) -> Result<sockaddr, NetError> {
        let mut address: libc::sockaddr = unsafe { core::mem::zeroed() };
        let mut address_len: libc::socklen_t =
            core::mem::size_of::<libc::sockaddr>() as libc::socklen_t;

        debug!("libc::getsockname(): sockfd={sockfd:?}");

        match unsafe { libc::getsockname(sockfd, &mut address, &mut address_len) } {
            -1 => {
                let errno: libc::c_int = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    return Err(NetError::Interrupted);
                }
                error!("libc::getsockname(): failed with errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno).unwrap_or_else(|_| {
                    warn!("unknown errno value {errno:?}, mapping to ValueOutOfRange");
                    ErrorCode::ValueOutOfRange
                });
                Err(NetError::Errno(error))
            },
            _ => {
                let mut sa_data: [u8; 14] = [0u8; 14];
                for (i, &b) in address.sa_data.iter().enumerate() {
                    sa_data[i] = b as u8;
                }
                let addr: sockaddr = sockaddr {
                    sa_len: address_len as u8,
                    sa_family: address.sa_family as u8,
                    sa_data,
                };
                Ok(addr)
            },
        }
    }
}
