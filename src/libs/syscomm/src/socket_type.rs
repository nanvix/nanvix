// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::log::error;
use ::num_enum::{
    IntoPrimitive,
    TryFromPrimitive,
};
use ::std::{
    io,
    str::FromStr,
};

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

impl SocketType {
    /// String representation for TCP sockets.
    pub const TCP_STR: &'static str = "tcp";

    /// String representation for Unix sockets.
    pub const UNIX_STR: &'static str = "unix";

    ///
    /// # Description
    ///
    /// Converts the socket type to a string.
    ///
    /// # Returns
    ///
    /// This function returns a string representation of the socket type.
    ///
    pub fn to_str(&self) -> &'static str {
        match self {
            SocketType::Tcp => Self::TCP_STR,
            SocketType::Unix => Self::UNIX_STR,
        }
    }
}

impl FromStr for SocketType {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            Self::TCP_STR => Ok(SocketType::Tcp),
            Self::UNIX_STR => Ok(SocketType::Unix),
            typ => {
                let reason: String = format!("unknown socket type '{typ}'");
                error!("from_str(): {reason}");
                Err(io::Error::new(io::ErrorKind::InvalidInput, reason))
            },
        }
    }
}
