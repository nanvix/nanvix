// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::cmp;

//==================================================================================================
// Constants
//==================================================================================================

/// Capacity of the home-directory buffer, including the terminating NUL byte.
pub(super) const PW_DIR_CAP: usize = 256;

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Returns the number of bytes of `home` that should be copied into the
/// fixed-size `pw_dir` buffer, reserving one byte for the terminating NUL.
///
/// The returned length never exceeds `PW_DIR_CAP - 1`, so callers can always
/// write a NUL terminator at the returned offset.
pub(super) fn pw_dir_copy_len(home: &[u8]) -> usize {
    cmp::min(home.len(), PW_DIR_CAP - 1)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests;
