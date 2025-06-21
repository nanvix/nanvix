// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::{
    String,
    ToString,
};

//==================================================================================================
// C Interface
//==================================================================================================

/// Represents a Unix socket address.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct SocketAddrUnix {
    /// Path.
    path: String,
}

impl SocketAddrUnix {
    /// Creates a new Unix socket address.
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    /// Gets the path of the Unix socket address.
    pub fn path(&self) -> &str {
        self.path.as_str()
    }
}

impl Clone for SocketAddrUnix {
    fn clone(&self) -> Self {
        Self {
            path: { self.path.clone() },
        }
    }
}
