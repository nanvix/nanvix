// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Descriptor backend route.

//==================================================================================================
// Enumerations
//==================================================================================================

/// The backend a descriptor's slot is bound to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VfsRoute {
    /// A console stream.
    Console,
    /// An object served by vfsd.
    Vfs,
    /// A socket served by networkd.
    Socket,
}
