// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::num_enum::{
    IntoPrimitive,
    TryFromPrimitive,
};
use ::std::{
    io,
    str::FromStr,
};
use ::syslog::error;

//==================================================================================================
// Structures
//==================================================================================================

/// An enum representing the type of a socket.
#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive, PartialEq)]
#[repr(u8)]
pub enum SocketType {
    /// TCP socket.
    Tcp,
    /// Unix socket.
    Unix,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl FromStr for SocketType {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tcp" => Ok(SocketType::Tcp),
            "unix" => Ok(SocketType::Unix),
            typ => {
                let reason: String = format!("unknown socket type '{typ}'");
                error!("from_str(): {reason}");
                Err(io::Error::new(io::ErrorKind::InvalidInput, reason))
            },
        }
    }
}
