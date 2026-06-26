// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::set_errno;
use ::sysapi::{
    errno::{
        EINVAL,
        ELOOP,
        ENAMETOOLONG,
        ENOENT,
        ENOTDIR,
    },
    ffi::{
        c_char,
        c_int,
    },
    limits::PATH_MAX,
    sys_stat::{
        self,
        file_type::{
            S_ISDIR,
            S_ISLNK,
        },
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of symbolic links to resolve before reporting a loop.
const SYMLOOP_MAX: u32 = 40;

//==================================================================================================
// Private Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the length of the null-terminated C string `s`, scanning at most `max` bytes.
///
/// # Returns
///
/// The length on success, or [`None`] if no terminator is found within `max` bytes.
///
/// # Safety
///
/// `s` must be readable up to and including its null terminator, or for at least `max` bytes.
///
unsafe fn cstr_len(s: *const c_char, max: usize) -> Option<usize> {
    // SAFETY: the caller guarantees `s` is readable; the scan never reads past `max` bytes.
    (0..max).find(|&i| unsafe { *s.add(i) } == 0)
}

///
/// # Description
///
/// Removes the trailing path component from the canonical path held in `out[..len]`.
///
/// # Returns
///
/// The new length. When `out[..len]` holds only the root directory (or is empty), the length
/// collapses to `0`, which this module treats as the root.
///
fn pop_component(out: &[u8], len: usize) -> usize {
    let mut i: usize = len;
    while i > 0 && out[i - 1] != b'/' {
        i -= 1;
    }
    // `i` now indexes just past the separator that precedes the component; drop that separator too.
    i.saturating_sub(1)
}

///
/// # Description
///
/// Appends a single path component to the canonical path held in `out[..*len]`.
///
/// # Returns
///
/// The previous length on success, or [`None`] if the component would overflow `out`.
///
fn append_component(comp: &[u8], out: &mut [u8], len: &mut usize) -> Option<usize> {
    let old_len: usize = *len;
    if *len + 1 + comp.len() >= out.len() {
        return None;
    }

    out[*len] = b'/';
    *len += 1;
    out[*len..*len + comp.len()].copy_from_slice(comp);
    *len += comp.len();
    Some(old_len)
}

///
/// # Description
///
/// Appends the path components found in `src` to the canonical path accumulated in `out`, resolving
/// `.` and `..` lexically and updating `len` in place.
///
/// # Returns
///
/// `true` on success, or `false` if `out` would overflow (leaving room for a null terminator).
///
fn append_components(src: &[u8], out: &mut [u8], len: &mut usize) -> bool {
    let n: usize = src.len();
    let mut start: usize = 0;
    let mut i: usize = 0;
    while i <= n {
        if i == n || src[i] == b'/' {
            let comp: &[u8] = &src[start..i];
            if comp.is_empty() || comp == b"." {
                // Skip empty components (from "//", or a leading/trailing "/") and ".".
            } else if comp == b".." {
                *len = pop_component(out, *len);
            } else {
                if append_component(comp, out, len).is_none() {
                    return false;
                }
            }
            start = i + 1;
        }
        i += 1;
    }
    true
}

///
/// # Description
///
/// Builds the lexically canonical absolute path of `path` into `out`, null-terminated. A relative
/// `path` is resolved against the absolute `cwd`.
///
/// # Returns
///
/// The length excluding the terminator, or [`None`] if the result does not fit in `out`.
///
#[cfg(all(test, feature = "std"))]
fn canonicalize(path: &[u8], cwd: &[u8], out: &mut [u8]) -> Option<usize> {
    let mut len: usize = 0;

    // A relative path is resolved against the current working directory.
    if path.first() != Some(&b'/') && !append_components(cwd, out, &mut len) {
        return None;
    }
    if !append_components(path, out, &mut len) {
        return None;
    }

    // An empty accumulator denotes the root directory.
    if len == 0 {
        if out.is_empty() {
            return None;
        }
        out[0] = b'/';
        len = 1;
    }

    if len >= out.len() {
        return None;
    }
    out[len] = 0;
    Some(len)
}

///
/// # Description
///
/// Builds the next path fragment to process after expanding a symbolic link.
///
/// # Returns
///
/// The length of the composed fragment, or [`None`] if it would exceed `PATH_MAX`.
///
fn compose_pending(target: &[u8], remaining: &[u8], out: &mut [u8]) -> Option<usize> {
    let total_len: usize = target.len().checked_add(remaining.len())?;
    if total_len >= out.len() {
        return None;
    }

    out[..target.len()].copy_from_slice(target);
    out[target.len()..total_len].copy_from_slice(remaining);
    Some(total_len)
}

///
/// # Description
///
/// Resolves `path` against `cwd`, expanding symbolic links and writing the canonical absolute path
/// to `out`.
///
/// # Returns
///
/// The length of the resolved path, excluding the null terminator, on success; [`None`] on failure.
/// When failure is due to local bounds or path-shape validation, this function sets `errno`.
/// Failures from `lstat()` and `readlink()` propagate their existing `errno` value.
///
/// # Safety
///
/// The platform `lstat()` and `readlink()` symbols must follow their C contracts. `path`, `cwd`,
/// and `out` are Rust slices and are therefore valid for their lengths.
///
unsafe fn resolve_path(path: &[u8], cwd: &[u8], out: &mut [u8]) -> Option<usize> {
    unsafe extern "C" {
        fn lstat(pathname: *const c_char, statbuf: *mut sys_stat::stat) -> c_int;
        fn readlink(path: *const c_char, buf: *mut c_char, bufsize: c_size_t) -> c_ssize_t;
    }

    let mut pending: [u8; PATH_MAX] = [0; PATH_MAX];
    if path.len() >= pending.len() {
        set_errno(ENAMETOOLONG);
        return None;
    }
    pending[..path.len()].copy_from_slice(path);
    let mut pending_len: usize = path.len();
    let mut cursor: usize = 0;

    let mut len: usize = 0;
    if path.first() != Some(&b'/') && !append_components(cwd, out, &mut len) {
        set_errno(ENAMETOOLONG);
        return None;
    }

    let mut symlinks: u32 = 0;
    while cursor < pending_len {
        while cursor < pending_len && pending[cursor] == b'/' {
            cursor += 1;
        }
        if cursor >= pending_len {
            break;
        }

        let start: usize = cursor;
        while cursor < pending_len && pending[cursor] != b'/' {
            cursor += 1;
        }
        let comp: &[u8] = &pending[start..cursor];

        if comp == b"." {
            continue;
        }
        if comp == b".." {
            len = pop_component(out, len);
            continue;
        }

        let Some(parent_len) = append_component(comp, out, &mut len) else {
            set_errno(ENAMETOOLONG);
            return None;
        };
        out[len] = 0;

        let mut statbuf: sys_stat::stat = sys_stat::stat::default();
        if unsafe { lstat(out.as_ptr().cast::<c_char>(), &mut statbuf) } != 0 {
            return None;
        }

        if S_ISLNK(statbuf.st_mode) {
            if symlinks >= SYMLOOP_MAX {
                set_errno(ELOOP);
                return None;
            }
            symlinks += 1;

            let mut target: [u8; PATH_MAX] = [0; PATH_MAX];
            // PATH_MAX (1024) trivially fits in c_size_t, so this cast cannot truncate.
            #[allow(clippy::cast_possible_truncation)]
            let target_len: c_ssize_t = unsafe {
                readlink(
                    out.as_ptr().cast::<c_char>(),
                    target.as_mut_ptr().cast::<c_char>(),
                    PATH_MAX as c_size_t,
                )
            };
            if target_len < 0 {
                return None;
            }

            let target_len: usize = match usize::try_from(target_len) {
                Ok(len) => len,
                Err(_) => {
                    set_errno(ENAMETOOLONG);
                    return None;
                },
            };
            if target_len == 0 || target_len >= PATH_MAX {
                set_errno(ENAMETOOLONG);
                return None;
            }

            len = parent_len;
            if target[0] == b'/' {
                len = 0;
            }

            let mut next_pending: [u8; PATH_MAX] = [0; PATH_MAX];
            let Some(next_pending_len) = compose_pending(
                &target[..target_len],
                &pending[cursor..pending_len],
                &mut next_pending,
            ) else {
                set_errno(ENAMETOOLONG);
                return None;
            };

            pending[..next_pending_len].copy_from_slice(&next_pending[..next_pending_len]);
            pending_len = next_pending_len;
            cursor = 0;
            continue;
        }

        if cursor < pending_len && !S_ISDIR(statbuf.st_mode) {
            set_errno(ENOTDIR);
            return None;
        }
    }

    if len == 0 {
        if out.is_empty() {
            set_errno(ENAMETOOLONG);
            return None;
        }
        out[0] = b'/';
        len = 1;
    }

    if len >= out.len() {
        set_errno(ENAMETOOLONG);
        return None;
    }
    out[len] = 0;
    Some(len)
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Resolves `path` to a canonical absolute pathname, removing `.` and `..` components, extra `/`
/// separators, and symbolic links. Relative paths are resolved against the current working
/// directory.
///
/// # Parameters
///
/// - `path`: Null-terminated path to resolve.
/// - `resolved_path`: Destination buffer of at least `PATH_MAX` bytes, or null to request a buffer
///   allocated with [`malloc`](crate::malloc) that the caller must `free()`.
///
/// # Returns
///
/// On success, a pointer to the resolved path (either `resolved_path` or the allocated buffer). On
/// failure, null is returned and `errno` is set (`EINVAL` if `path` is null, `ENOENT` if `path` is
/// empty or names no existing file, or `ENAMETOOLONG` if the result exceeds `PATH_MAX`).
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that `path`
/// points to a valid null-terminated string and that `resolved_path`, when non-null, points to a
/// writable buffer of at least `PATH_MAX` bytes.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/realpath.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn realpath(path: *const c_char, resolved_path: *mut c_char) -> *mut c_char {
    unsafe extern "C" {
        fn getcwd(buf: *mut c_char, size: c_size_t) -> *mut c_char;
    }

    // A null `path` is invalid.
    if path.is_null() {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    }

    // Read the input path, bounding the scan by PATH_MAX.
    // SAFETY: `path` is non-null and the caller guarantees it is null-terminated.
    let Some(path_len) = (unsafe { cstr_len(path, PATH_MAX) }) else {
        set_errno(ENAMETOOLONG);
        return core::ptr::null_mut();
    };

    // An empty path names no file.
    if path_len == 0 {
        set_errno(ENOENT);
        return core::ptr::null_mut();
    }

    // SAFETY: `path` is readable for `path_len` bytes, as just established by `cstr_len()`.
    let path_bytes: &[u8] = unsafe { core::slice::from_raw_parts(path.cast::<u8>(), path_len) };

    // Resolve relative paths against the current working directory.
    let mut cwd_buf: [u8; PATH_MAX] = [0; PATH_MAX];
    let cwd_bytes: &[u8] = if path_bytes[0] == b'/' {
        &[]
    } else {
        // SAFETY: `cwd_buf` is a valid writable buffer of PATH_MAX bytes.
        // PATH_MAX (1024) trivially fits in c_size_t, so the width-narrowing cast cannot truncate.
        #[allow(clippy::cast_possible_truncation)]
        let ret: *mut c_char =
            unsafe { getcwd(cwd_buf.as_mut_ptr().cast::<c_char>(), PATH_MAX as c_size_t) };
        if ret.is_null() {
            // getcwd() has set errno.
            return core::ptr::null_mut();
        }
        // SAFETY: getcwd() wrote a null-terminated path into `cwd_buf`.
        let Some(cwd_len) = (unsafe { cstr_len(cwd_buf.as_ptr().cast::<c_char>(), PATH_MAX) })
        else {
            set_errno(ENAMETOOLONG);
            return core::ptr::null_mut();
        };
        &cwd_buf[..cwd_len]
    };

    // Canonicalize the path and resolve symbolic links.
    let mut out: [u8; PATH_MAX] = [0; PATH_MAX];
    let Some(out_len) = (unsafe { resolve_path(path_bytes, cwd_bytes, &mut out) }) else {
        return core::ptr::null_mut();
    };

    // Select the destination: the caller's buffer, or a freshly allocated one.
    let dst: *mut c_char = if resolved_path.is_null() {
        // SAFETY: request out_len + 1 bytes for the path and its terminator.
        // out_len < PATH_MAX (1024), so out_len + 1 fits in c_size_t and the cast cannot truncate.
        #[allow(clippy::cast_possible_truncation)]
        let p = unsafe { crate::malloc::malloc((out_len + 1) as c_size_t) };
        if p.is_null() {
            // malloc() has set errno to ENOMEM.
            return core::ptr::null_mut();
        }
        p.cast::<c_char>()
    } else {
        resolved_path
    };

    // Copy the canonical path, including its null terminator, into the destination.
    // SAFETY: `dst` has room for out_len + 1 bytes (PATH_MAX for the caller's buffer, or the exact
    // allocation above); `out` holds out_len + 1 initialized bytes.
    unsafe {
        core::ptr::copy_nonoverlapping(out.as_ptr(), dst.cast::<u8>(), out_len + 1);
    }

    dst
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::canonicalize;
    use ::std::vec::Vec;

    /// Canonicalizes `path` against `cwd`, returning the resulting bytes.
    fn run(path: &str, cwd: &str) -> Option<Vec<u8>> {
        let mut out: [u8; 1024] = [0; 1024];
        let len: usize = canonicalize(path.as_bytes(), cwd.as_bytes(), &mut out)?;
        Some(out[..len].to_vec())
    }

    #[test]
    fn test_absolute_dotdot() {
        assert_eq!(run("/tmp/../tmp/hello.txt", "/").as_deref(), Some(&b"/tmp/hello.txt"[..]));
    }

    #[test]
    fn test_relative_against_cwd() {
        assert_eq!(run("a/b", "/home/u").as_deref(), Some(&b"/home/u/a/b"[..]));
    }

    #[test]
    fn test_dot_and_double_slash() {
        assert_eq!(run("/a/./b//c", "/").as_deref(), Some(&b"/a/b/c"[..]));
    }

    #[test]
    fn test_dotdot_past_root_stays_root() {
        assert_eq!(run("/a/../../x", "/").as_deref(), Some(&b"/x"[..]));
    }

    #[test]
    fn test_root() {
        assert_eq!(run("/", "/").as_deref(), Some(&b"/"[..]));
        assert_eq!(run("/..", "/").as_deref(), Some(&b"/"[..]));
    }

    #[test]
    fn test_trailing_slash_and_dot() {
        assert_eq!(run("/a/b/.", "/").as_deref(), Some(&b"/a/b"[..]));
        assert_eq!(run("/a/b/", "/").as_deref(), Some(&b"/a/b"[..]));
    }

    #[test]
    fn test_overflow_returns_none() {
        let mut out: [u8; 8] = [0; 8];
        assert_eq!(canonicalize(b"/aaaaaaaaaaaaaaaa", b"/", &mut out), None);
    }
}
