// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Re-exports
//==================================================================================================

// Re-export constants that socket2 expects to find directly in sys module
#[cfg(feature = "networking")]
pub use crate::{
    netinet::in_::bindings::{
        ipproto::IPPROTO_IPV6,
        IPV6_RECVHOPLIMIT,
        IP_HDRINCL,
    },
    sys::socket::{
        SOCK_RAW,
        SOCK_SEQPACKET,
    },
};

//==================================================================================================
// Modules
//==================================================================================================

/// Memory management operations.
pub mod mman;

/// Synchronous I/O multiplexing.
pub mod select;

/// Sockets.
#[cfg(feature = "networking")]
pub mod socket;

/// File status.
pub mod stat;

/// Time types.
pub mod time;

/// File access and modification times structure.
pub mod times;

/// Vector I/O operations.
pub mod uio;

/// Definitions for UNIX domain sockets.
#[cfg(feature = "networking")]
pub mod un;

/// System name structure.
pub mod utsname;
