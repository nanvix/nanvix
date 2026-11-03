// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
