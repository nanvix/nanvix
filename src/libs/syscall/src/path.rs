// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Client-Side Path Expansion
//==================================================================================================

use ::alloc::borrow::Cow;

/// Performs client-side expansion of shell-style path prefixes.
///
/// Currently expands a leading `~` or `~/` using the `HOME` environment variable from the
/// process-local environment table. Designed to be extended with additional expansions (e.g.,
/// `$VAR` / `${VAR}`) in the future without changing call sites.
///
/// # Standalone Mode
///
/// In standalone mode, VFS state is owned by vfsd and the daemon has no knowledge of per-process
/// environment variables. Tilde expansion is therefore performed client-side before the path is
/// serialized into an IPC message.
///
/// # Non-Standalone Mode
///
/// In non-standalone (hosted) mode, paths are forwarded to linuxd which performs its own
/// resolution. This function returns the path unchanged.
///
/// # Parameters
///
/// - `path`: The path to expand.
///
/// # Returns
///
/// A [`Cow::Borrowed`] reference to the original path when no expansion is needed, or a
/// [`Cow::Owned`] string with `~` replaced by `$HOME`.
///
#[cfg(feature = "standalone")]
pub(crate) fn expand_path(path: &str) -> Cow<'_, str> {
    expand_path_with_home(path, &home_dir())
}

/// Returns the path unchanged in non-standalone mode.
#[cfg(not(feature = "standalone"))]
pub(crate) fn expand_path(path: &str) -> Cow<'_, str> {
    Cow::Borrowed(path)
}

/// Core tilde-expansion logic, parameterized by the home directory string.
///
/// Expands `~` to `home` and `~/...` to `home/...`. All other paths are returned unchanged.
#[cfg(feature = "standalone")]
fn expand_path_with_home<'a>(path: &'a str, home: &str) -> Cow<'a, str> {
    if path == "~" {
        Cow::Owned(alloc::string::String::from(home))
    } else if let Some(rest) = path.strip_prefix("~/") {
        let trimmed: &str = home.trim_end_matches('/');
        Cow::Owned(alloc::format!("{}/{}", trimmed, rest))
    } else {
        Cow::Borrowed(path)
    }
}

/// Reads the `HOME` environment variable from the process-local environment table.
///
/// Falls back to `"/"` when `HOME` is not set or contains invalid UTF-8.
#[cfg(feature = "standalone")]
fn home_dir() -> alloc::string::String {
    let ptr: *const ::sysapi::ffi::c_char = ::libc_stdlib::env_table::get("HOME");
    if ptr.is_null() {
        return alloc::string::String::from("/");
    }
    // SAFETY: `env_table::get` returns a pointer into a live `EnvEntry::raw` buffer that remains
    // valid as long as no concurrent `set`/`unset` modifies the same key. This matches POSIX
    // `getenv()` semantics.
    let c_str: &core::ffi::CStr = unsafe { core::ffi::CStr::from_ptr(ptr) };
    match c_str.to_str() {
        Ok(s) if !s.is_empty() => alloc::string::String::from(s),
        _ => alloc::string::String::from("/"),
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- expand_path_with_home tests (standalone only) ---------------------------
    //
    // `expand_path_with_home` is gated behind `#[cfg(feature = "standalone")]`, so these
    // tests must be gated the same way to avoid compilation errors in non-standalone builds.

    #[cfg(feature = "standalone")]
    mod standalone {
        use super::*;

        /// Tests that bare "~" expands to the home directory.
        #[test]
        fn expand_bare_tilde() {
            let result = expand_path_with_home("~", "/home/user");
            assert_eq!(result, "/home/user");
        }

        /// Tests that "~/subdir" expands to "$HOME/subdir".
        #[test]
        fn expand_tilde_subpath() {
            let result = expand_path_with_home("~/docs/file.txt", "/home/user");
            assert_eq!(result, "/home/user/docs/file.txt");
        }

        /// Tests that a trailing slash in home is not duplicated.
        #[test]
        fn expand_tilde_home_trailing_slash() {
            let result = expand_path_with_home("~/file.txt", "/home/user/");
            assert_eq!(result, "/home/user/file.txt");
        }

        /// Tests that bare "~" with root home returns "/".
        #[test]
        fn expand_bare_tilde_root_home() {
            let result = expand_path_with_home("~", "/");
            assert_eq!(result, "/");
        }

        /// Tests that "~/foo" with root home returns "/foo" (no double slash).
        #[test]
        fn expand_tilde_subpath_root_home() {
            let result = expand_path_with_home("~/foo", "/");
            assert_eq!(result, "/foo");
        }

        /// Tests that absolute paths are returned unchanged.
        #[test]
        fn expand_absolute_path_unchanged() {
            let result = expand_path_with_home("/data/file.txt", "/home/user");
            assert_eq!(result, "/data/file.txt");
        }

        /// Tests that relative paths without tilde are returned unchanged.
        #[test]
        fn expand_relative_path_unchanged() {
            let result = expand_path_with_home("data/file.txt", "/home/user");
            assert_eq!(result, "data/file.txt");
        }

        /// Tests that "~user" form is not expanded (Nanvix is single-user).
        #[test]
        fn expand_tilde_user_not_expanded() {
            let result = expand_path_with_home("~other", "/home/user");
            assert_eq!(result, "~other");
        }

        /// Tests that "~user/path" form is not expanded.
        #[test]
        fn expand_tilde_user_subpath_not_expanded() {
            let result = expand_path_with_home("~other/docs", "/home/user");
            assert_eq!(result, "~other/docs");
        }
    }

    /// Tests that the non-standalone expand_path returns path unchanged.
    #[cfg(not(feature = "standalone"))]
    #[test]
    fn expand_path_non_standalone_passthrough() {
        let result = expand_path("~/foo");
        assert_eq!(result, "~/foo");
    }
}
