// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Maximum number of [`crate::sys::uio::iovec`] structures that can be passed to a single call to
/// [`crate::sys::uio::writev`] or [`crate::sys::uio::readv`].
pub const IOV_MAX: usize = 16;

/// Maximum number of bytes in a filename (not including the terminating null byte).
pub const NAME_MAX: usize = 16;

/// Maximum number of bytes the implementation stores as a pathname in a user-supplied buffer of
/// unspecified size, including the terminating null character.
pub const PATH_MAX: usize = 256;

/// Maximum value for an object of type [`crate::sys::types::ssize_t`].
pub const SSIZE_MAX: crate::sys::types::ssize_t = crate::sys::types::ssize_t::MAX;
