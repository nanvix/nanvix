// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Descriptor backend classification, independent of the resolution cache.

//==================================================================================================
// Enumerations
//==================================================================================================

/// The backend that serves a descriptor's operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    /// A console stream (`stdin`/`stdout`/`stderr`); I/O flows directly to the kernel.
    Console,
    /// A terminal device whose I/O is served by vfsd.
    Terminal,
    /// A `vfsd`-managed object: regular file, directory, host file, or pipe end.
    Vfs,
    /// A `networkd`-managed socket.
    Socket,
}
