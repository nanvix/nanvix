// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::socket::sa_family_t;
use ::alloc::string::{
    String,
    ToString,
};
use ::core::mem;

//==================================================================================================
// C Interface
//==================================================================================================

pub mod bindings {

    #![allow(non_camel_case_types)]

    use super::*;

    /// Size of the `sun_path` field in [`sockaddr_un`].
    pub const SUNPATHLEN: usize = 14;

    /// Describes a UNIX domain socket address.
    #[repr(C, packed)]
    pub struct sockaddr_un {
        /// Address family.
        pub sun_family: sa_family_t,
        /// Path.
        pub sun_path: [u8; SUNPATHLEN],
    }
    ::nvx::sys::static_assert_size!(sockaddr_un, sockaddr_un::SIZE);

    impl sockaddr_un {
        /// Size of the structure.
        pub const SIZE: usize = mem::size_of::<sa_family_t>() + SUNPATHLEN;
    }
}

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
