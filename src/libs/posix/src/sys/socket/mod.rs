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
            bind,
            listen,
            accept,
            shutdown,
            recv,
            send,
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

/// Disables further receive operations.
pub const SHUT_RD: i32 = 0;
/// Disables further send operations.
pub const SHUT_WR: i32 = 1;
/// Disables further send and receive operations.
pub const SHUT_RDWR: i32 = 2;

/// Peeks at an incoming message.
pub const MSG_PEEK: i32 = 0x2;
/// Requests out-of-band data.
pub const MSG_OOB: i32 = 0x1;
/// Requests to block until the full amount of data can be returned.
pub const MSG_WAITALL: i32 = 0x100;
/// Terminates a record.
pub const MSG_EOR: i32 = 0x8;
/// Requests not to send SIGPIPE on errors.
pub const MSG_NOSIGNAL: i32 = 0x4000;

/// Used for socket length.
pub type socklen_t = u32;

/// Used for socket address family.
pub type sa_family_t = u16;

/// Describes the address of a socket.
#[derive(Debug, Clone)]
#[repr(C, packed)]
pub struct sockaddr {
    /// Address family.
    pub sa_family: sa_family_t,
    /// Address data.
    pub sa_data: [u8; 14],
}
::nvx::sys::static_assert_size!(sockaddr, 16);
