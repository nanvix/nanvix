// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Maximum number of bytes in a filename (not including the terminating null byte).
pub const NAME_MAX: usize = 16;

/// Maximum number of bytes the implementation stores as a pathname in a user-supplied buffer of
/// unspecified size, including the terminating null character.
pub const PATH_MAX: usize = 256;
