// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod message;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use self::syscall::{
            socket,
            bind
        };
    }
}

//==================================================================================================

/// Internet domain sockets for use with IPv4 addresses.
pub const AF_INET: sa_family_t = 2;
/// Internet domain sockets for use with IPv6 addresses.
pub const AF_INET6: sa_family_t = 10;
/// Unix domain sockets.
pub const AF_UNIX: sa_family_t = 1;
/// Unspecified.
pub const AF_UNSPEC: sa_family_t = 0;

/// Provides sequenced, reliable, bidirectional, connection-mode byte streams.
pub const SOCK_STREAM: i32 = 1;
/// Provides raw network protocol access.
pub const SOCK_RAW: i32 = 3;
/// Provides datagrams, which are connectionless-mode, unreliable messages of fixed maximum length.
pub const SOCK_DGRAM: i32 = 2;
/// Provides sequenced, reliable, bidirectional, connection-mode transmission paths for records.
pub const SOCK_SEQPACKET: i32 = 5;

/// Used for socket length.
pub type socklen_t = u32;

/// Used for socket address family.
pub type sa_family_t = u16;

/// Describes the address of a socket.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct sockaddr {
    /// Address family.
    pub sa_family: sa_family_t,
    /// Address data.
    pub sa_data: [u8; 14],
}
::nvx::sys::static_assert_size!(sockaddr, 16);
