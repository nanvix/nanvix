// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![deny(clippy::all)]

#[cfg(test)]
extern crate alloc;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Splits a packed command-line buffer in place, resolving `\;` escapes to literal `;`.
///
/// The format is `<app args>;<env vars>;<kernel args>`. The first unescaped `;` separates
/// application arguments from environment variables; the second unescaped `;` separates
/// environment variables from kernel arguments. Kernel arguments are a space-separated string
/// that the kernel uses to enable/disable internal features.
///
/// The sequence `\;` is compacted to a literal `;`. A backslash not followed by `;` is kept
/// verbatim.
///
/// Because unescaping only ever shortens the content, the compaction is performed in place with no
/// heap allocation.
///
/// # Panics
///
/// Panics if the compacted buffer is not valid UTF-8. This cannot happen when the original input
/// was valid UTF-8 because the compaction only removes ASCII `\` bytes that precede ASCII `;`.
///
/// # Parameters
///
/// - `buf`: Mutable byte slice containing the packed command line. Modified in place.
///   Must contain valid UTF-8.
///
/// # Returns
///
/// A tuple `(args, env, kernel_args)` as string slices into the compacted buffer.
pub fn split_cmdline(buf: &mut [u8]) -> (&str, &str, &str) {
    let len: usize = buf.len();
    let mut r: usize = 0;
    let mut w: usize = 0;

    // Phase 1: compact args, stop at the first unescaped `;`.
    while r < len {
        if buf[r] == b'\\' && r + 1 < len && buf[r + 1] == b';' {
            buf[w] = b';';
            w += 1;
            r += 2;
        } else if buf[r] == b';' {
            // First unescaped `;` → separator between args and env.
            let args_end: usize = w;
            r += 1;

            // Phase 2: compact env, stop at the second unescaped `;`.
            while r < len {
                if buf[r] == b'\\' && r + 1 < len && buf[r + 1] == b';' {
                    buf[w] = b';';
                    w += 1;
                    r += 2;
                } else if buf[r] == b';' {
                    // Second unescaped `;` → separator between env and kernel args.
                    let env_end: usize = w;
                    r += 1;

                    // Phase 3: compact kernel args after env.
                    while r < len {
                        if buf[r] == b'\\' && r + 1 < len && buf[r + 1] == b';' {
                            buf[w] = b';';
                            w += 1;
                            r += 2;
                        } else {
                            buf[w] = buf[r];
                            w += 1;
                            r += 1;
                        }
                    }

                    let buf: &[u8] = buf;
                    let args: &str = unwrap_utf8(&buf[..args_end]);
                    let env: &str = unwrap_utf8(&buf[args_end..env_end]);
                    let kernel_args: &str = unwrap_utf8(&buf[env_end..w]);
                    return (args, env, kernel_args);
                } else {
                    buf[w] = buf[r];
                    w += 1;
                    r += 1;
                }
            }

            // No second separator — no kernel args.
            let buf: &[u8] = buf;
            let args: &str = unwrap_utf8(&buf[..args_end]);
            let env: &str = unwrap_utf8(&buf[args_end..w]);
            return (args, env, "");
        } else {
            buf[w] = buf[r];
            w += 1;
            r += 1;
        }
    }

    // No separator found — entire buffer is args.
    let args: &str = unwrap_utf8(&buf[..w]);
    (args, "", "")
}

/// Converts a byte slice to `&str`, panicking on invalid UTF-8.
fn unwrap_utf8(bytes: &[u8]) -> &str {
    core::str::from_utf8(bytes).expect("cmdline: invalid UTF-8 after compaction")
}

///
/// # Description
///
/// Finds the byte offset of the second unescaped `;` in a command-line string.
///
/// This is used to locate the boundary between `<args>;<env>` and `<kernel_args>` without
/// modifying the buffer. The sequence `\;` is treated as an escaped literal and is not counted
/// as a separator.
///
/// # Parameters
///
/// - `s`: The command-line string to scan.
///
/// # Returns
///
/// `Some(offset)` where `offset` is the byte index of the second unescaped `;`, or `None` if
/// fewer than two unescaped semicolons are present.
pub fn find_kernel_args_start(s: &str) -> Option<usize> {
    let bytes: &[u8] = s.as_bytes();
    let mut semicolons: usize = 0;
    let mut i: usize = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b';' {
            i += 2;
        } else if bytes[i] == b';' {
            semicolons += 1;
            if semicolons == 2 {
                return Some(i);
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    None
}

///
/// # Description
///
/// Compacts `\;` escape sequences to literal `;` in a byte buffer, in place.
///
/// This is used to unescape a single section of a command-line string (e.g., the kernel
/// arguments tail) without performing a full `split_cmdline` parse.
///
/// # Panics
///
/// Panics if the compacted buffer is not valid UTF-8.
///
/// # Parameters
///
/// - `buf`: Mutable byte slice to compact in place. Must contain valid UTF-8.
///
/// # Returns
///
/// A `&str` slice into the compacted portion of `buf`.
pub fn compact_semicolon_escapes(buf: &mut [u8]) -> &str {
    let len: usize = buf.len();
    let mut r: usize = 0;
    let mut w: usize = 0;
    while r < len {
        if buf[r] == b'\\' && r + 1 < len && buf[r + 1] == b';' {
            buf[w] = b';';
            w += 1;
            r += 2;
        } else {
            buf[w] = buf[r];
            w += 1;
            r += 1;
        }
    }
    unwrap_utf8(&buf[..w])
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: copies the input into a mutable buffer and calls `split_cmdline`.
    fn split(
        input: &str,
    ) -> (::alloc::string::String, ::alloc::string::String, ::alloc::string::String) {
        let mut buf: ::alloc::vec::Vec<u8> = input.as_bytes().to_vec();
        let (args, env, kargs) = split_cmdline(&mut buf);
        (
            ::alloc::string::String::from(args),
            ::alloc::string::String::from(env),
            ::alloc::string::String::from(kargs),
        )
    }

    #[test]
    fn args_only() {
        let (args, env, kargs) = split("arg1 arg2");
        assert_eq!(args, "arg1 arg2", "args mismatch");
        assert_eq!(env, "", "env should be empty");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    #[test]
    fn args_and_env() {
        let (args, env, kargs) = split("arg1 arg2;VAR1=foo VAR2=bar");
        assert_eq!(args, "arg1 arg2", "args mismatch");
        assert_eq!(env, "VAR1=foo VAR2=bar", "env mismatch");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    #[test]
    fn env_only() {
        let (args, env, kargs) = split(";VAR1=foo");
        assert_eq!(args, "", "args should be empty");
        assert_eq!(env, "VAR1=foo", "env mismatch");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    #[test]
    fn empty_string() {
        let (args, env, kargs) = split("");
        assert_eq!(args, "", "args should be empty");
        assert_eq!(env, "", "env should be empty");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    #[test]
    fn escaped_semicolon_in_args() {
        let (args, env, kargs) = split("arg1 with\\;semicolon arg2;VAR1=foo");
        assert_eq!(args, "arg1 with;semicolon arg2", "escaped \\; should become ;");
        assert_eq!(env, "VAR1=foo", "env mismatch");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    #[test]
    fn escaped_semicolon_in_env() {
        let (args, env, kargs) = split("arg1;VAR=a\\;b");
        assert_eq!(args, "arg1", "args mismatch");
        assert_eq!(env, "VAR=a;b", "escaped \\; in env should become ;");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    #[test]
    fn multiple_escaped_semicolons() {
        let (args, env, kargs) = split("a\\;b\\;c;VAR=d\\;e");
        assert_eq!(args, "a;b;c", "multiple escaped semicolons");
        assert_eq!(env, "VAR=d;e", "env escaped semicolon");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    #[test]
    fn escaped_only_no_separator() {
        let (args, env, kargs) = split("a\\;b");
        assert_eq!(args, "a;b", "escaped semicolon without separator");
        assert_eq!(env, "", "env should be empty");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    #[test]
    fn backslash_before_non_semicolon() {
        let (args, env, kargs) = split("C:\\path\\file;VAR=x");
        assert_eq!(args, "C:\\path\\file", "lone backslashes preserved");
        assert_eq!(env, "VAR=x", "env mismatch");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    #[test]
    fn backslash_at_end() {
        let (args, env, kargs) = split("trail\\");
        assert_eq!(args, "trail\\", "trailing backslash preserved");
        assert_eq!(env, "", "env should be empty");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    #[test]
    fn backslash_semicolon_then_separator() {
        let (args, env, kargs) = split("a\\;;VAR=x");
        assert_eq!(args, "a;", "backslash-semicolon then separator");
        assert_eq!(env, "VAR=x", "env after separator");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    #[test]
    fn backward_compatible_no_semicolons() {
        let (args, env, kargs) = split("hello world");
        assert_eq!(args, "hello world", "plain args unchanged");
        assert_eq!(env, "", "env should be empty");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    #[test]
    fn backward_compatible_with_separator() {
        let (args, env, kargs) = split("prog;HOME=/tmp");
        assert_eq!(args, "prog", "args before separator");
        assert_eq!(env, "HOME=/tmp", "env after separator");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    #[test]
    fn all_three_components() {
        let (args, env, kargs) = split("arg1 arg2;VAR1=foo;feature1 feature2");
        assert_eq!(args, "arg1 arg2", "args mismatch");
        assert_eq!(env, "VAR1=foo", "env mismatch");
        assert_eq!(kargs, "feature1 feature2", "kernel args mismatch");
    }

    #[test]
    fn kernel_args_only() {
        let (args, env, kargs) = split(";;feature1");
        assert_eq!(args, "", "args should be empty");
        assert_eq!(env, "", "env should be empty");
        assert_eq!(kargs, "feature1", "kernel args mismatch");
    }

    #[test]
    fn args_and_kernel_args_no_env() {
        let (args, env, kargs) = split("arg1;;feature1 feature2");
        assert_eq!(args, "arg1", "args mismatch");
        assert_eq!(env, "", "env should be empty");
        assert_eq!(kargs, "feature1 feature2", "kernel args mismatch");
    }

    #[test]
    fn env_and_kernel_args_no_app_args() {
        let (args, env, kargs) = split(";VAR=x;feature1");
        assert_eq!(args, "", "args should be empty");
        assert_eq!(env, "VAR=x", "env mismatch");
        assert_eq!(kargs, "feature1", "kernel args mismatch");
    }

    #[test]
    fn escaped_semicolon_in_kernel_args() {
        let (args, env, kargs) = split("arg1;VAR=x;feat\\;ure1 feature2");
        assert_eq!(args, "arg1", "args mismatch");
        assert_eq!(env, "VAR=x", "env mismatch");
        assert_eq!(kargs, "feat;ure1 feature2", "escaped \\; in kernel args should become ;");
    }

    #[test]
    fn empty_kernel_args_with_trailing_separator() {
        let (args, env, kargs) = split("arg1;VAR=x;");
        assert_eq!(args, "arg1", "args mismatch");
        assert_eq!(env, "VAR=x", "env mismatch");
        assert_eq!(kargs, "", "kernel args should be empty");
    }

    // ── find_kernel_args_start tests ─────────────────────────────────────────

    #[test]
    fn find_kargs_no_semicolons() {
        assert_eq!(find_kernel_args_start("hello world"), None);
    }

    #[test]
    fn find_kargs_one_semicolon() {
        assert_eq!(find_kernel_args_start("arg1;VAR=foo"), None);
    }

    #[test]
    fn find_kargs_two_semicolons() {
        // "arg1;VAR=foo;feature1"
        //  0123456789012345678901
        //              ^ position 12
        assert_eq!(find_kernel_args_start("arg1;VAR=foo;feature1"), Some(12));
    }

    #[test]
    fn find_kargs_escaped_semicolons_skipped() {
        // "a\\;b;VAR=x;feat" — the `\;` at position 1 is escaped, not a separator.
        // First real `;` is at position 4, second at position 10.
        assert_eq!(find_kernel_args_start("a\\;b;VAR=x;feat"), Some(10));
    }

    #[test]
    fn find_kargs_empty_sections() {
        // ";;feature1" — two separators at positions 0 and 1.
        assert_eq!(find_kernel_args_start(";;feature1"), Some(1));
    }

    #[test]
    fn find_kargs_all_escaped() {
        // "a\\;b\\;c" — no real separators.
        assert_eq!(find_kernel_args_start("a\\;b\\;c"), None);
    }
}
