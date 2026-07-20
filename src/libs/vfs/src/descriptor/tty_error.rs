// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Terminal-control query error.

//==================================================================================================
// Enumerations
//==================================================================================================

/// Error from a terminal-control query on a descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TtyError {
    /// The descriptor has no slot in the current process (maps to `EBADF`).
    BadFd,
    /// The descriptor is valid but does not refer to a terminal (maps to `ENOTTY`).
    NotTty,
}
