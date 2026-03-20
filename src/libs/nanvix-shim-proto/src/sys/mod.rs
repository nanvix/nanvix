// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Platform-specific shim protocol implementations.
//!
//! On Unix: uses Unix domain sockets and dup2 for stdout signaling.
//! On Windows: uses named pipes and handle manipulation.

#[cfg_attr(unix, path = "unix/mod.rs")]
#[cfg_attr(windows, path = "windows/mod.rs")]
mod platform;

pub use platform::{
    create_listener,
    parse_sockaddr,
    signal_server_started,
    socket_address,
};
