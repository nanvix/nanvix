// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::cmp;
use ::sysapi::sys_types::gid_t;

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
// resolve_gid()
//==================================================================================================

/// Resolves the real group ID of the calling process, falling back to `0` (root)
/// when the underlying `getgid()` lookup fails.
pub(super) fn resolve_gid() -> gid_t {
    match ::syscall::unistd::getgid() {
        Ok(gid) => gid,
        Err(error) => {
            ::syslog::warn!("getpwuid(): getgid() failed (error={:?})", error);
            0
        },
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// A short home path is reported by its exact length.
    #[test]
    fn copy_len_short_path() {
        assert_eq!(pw_dir_copy_len(b"/home/user"), b"/home/user".len());
    }

    /// The root fallback path has length one.
    #[test]
    fn copy_len_root() {
        assert_eq!(pw_dir_copy_len(b"/"), 1);
    }

    /// An over-long path is truncated to leave room for the NUL terminator.
    #[test]
    fn copy_len_truncates_oversized_path() {
        let long: [u8; PW_DIR_CAP + 16] = [b'a'; PW_DIR_CAP + 16];
        assert_eq!(pw_dir_copy_len(&long), PW_DIR_CAP - 1);
    }

    /// A path exactly filling the buffer (minus NUL) is not truncated.
    #[test]
    fn copy_len_exact_fit() {
        let exact: [u8; PW_DIR_CAP - 1] = [b'b'; PW_DIR_CAP - 1];
        assert_eq!(pw_dir_copy_len(&exact), PW_DIR_CAP - 1);
    }
}
