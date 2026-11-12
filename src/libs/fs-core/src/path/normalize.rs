// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Lexical path normalization.
//!
//! Normalizers rewrite only the spelling of a path: they neither access a filesystem nor anchor
//! relative paths, so absolute paths stay absolute and relative paths stay relative. Both apply the
//! rules of [POSIX.1-2024 §4.16] that hold in every namespace: successive separators are equivalent
//! to one, and a `.` component refers to its predecessor directory. Both also treat a leading `//`
//! as `/` and `..` in the root directory as the root itself, two choices that POSIX leaves to the
//! implementation. Only [`normalize_lexically`] folds `name/..` and drops trailing separators,
//! which can change the entry that a path names.
//!
//! [POSIX.1-2024 §4.16]: https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html#tag_04_16

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::{
    string::String,
    vec::Vec,
};

//==================================================================================================
// Structures
//==================================================================================================

/// Rewrites applied by [`normalize_with`].
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rewrites {
    /// Rewrites that keep the entry a path names in any namespace.
    Preserving,
    /// Preserving rewrites, plus folding `name/..` and dropping trailing separators.
    Lexical,
}

//==================================================================================================
// Public Functions
//==================================================================================================

/// Normalizes the spelling of `path` without changing the entry that it names.
///
/// Successive separators collapse into one, `.` components are removed, and `..` components in the
/// root directory are dropped. Every other `..` is kept, because the name before it may be a
/// symbolic link to a directory elsewhere, whose parent differs from the directory holding the
/// link. If `path` ends with a separator or with a `.` component, one trailing separator is kept,
/// so that the final entry must still be a directory.
///
/// # Parameters
///
/// - `path`: Path to normalize.
///
/// # Returns
///
/// The normalized path, which is `/` for the root directory and `.` for a relative path with no
/// remaining components. An empty `path` names no entry and is returned unchanged for the caller to
/// reject.
pub fn normalize(path: &str) -> String {
    normalize_with(path, Rewrites::Preserving)
}

/// Normalizes `path` as if none of its components were a symbolic link.
///
/// Besides the rewrites of [`normalize`], each `..` removes the name before it, and trailing
/// separators are dropped. A `..` with no name to remove is dropped in the root directory and kept
/// at the start of a relative path. The result names the same entry as `path` only if no component
/// is a symbolic link and every name followed by `..` is a directory. Callers that honor trailing
/// separators must check for them in `path` itself.
///
/// # Parameters
///
/// - `path`: Path to normalize.
///
/// # Returns
///
/// The normalized path, which is `/` for the root directory and `.` for a relative path with no
/// remaining components. An empty `path` names no entry and is returned unchanged for the caller to
/// reject.
pub fn normalize_lexically(path: &str) -> String {
    normalize_with(path, Rewrites::Lexical)
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Normalizes `path`, applying `rewrites`.
fn normalize_with(path: &str, rewrites: Rewrites) -> String {
    if path.is_empty() {
        return String::new();
    }

    let is_absolute: bool = path.starts_with('/');
    let mut components: Vec<&str> = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => {},
            ".." => match components.last() {
                Some(&previous) if rewrites == Rewrites::Lexical && previous != ".." => {
                    components.pop();
                },
                // The root directory is its own parent.
                None if is_absolute => {},
                _ => components.push(component),
            },
            name => components.push(name),
        }
    }

    // A trailing separator or a final `.` component requires the final entry to be a directory.
    let requires_directory: bool = matches!(path.rsplit('/').next(), Some("" | "."));

    let mut normalized: String = String::with_capacity(path.len());
    if is_absolute {
        normalized.push('/');
    } else if components.is_empty() {
        normalized.push('.');
    }
    for (index, component) in components.iter().enumerate() {
        if index > 0 {
            normalized.push('/');
        }
        normalized.push_str(component);
    }
    if rewrites == Rewrites::Preserving && requires_directory && !components.is_empty() {
        normalized.push('/');
    }

    normalized
}
