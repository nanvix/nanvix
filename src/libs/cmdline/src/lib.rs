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
/// The format is `<app args>;<env vars>`. The first unescaped `;` separates application arguments
/// from environment variables.
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
/// A tuple `(args, env)` as string slices into the compacted buffer.
pub fn split_cmdline(buf: &mut [u8]) -> (&str, &str) {
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

            // Phase 2: compact env (remainder of buffer).
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
            let env: &str = unwrap_utf8(&buf[args_end..w]);
            return (args, env);
        } else {
            buf[w] = buf[r];
            w += 1;
            r += 1;
        }
    }

    // No separator found — entire buffer is args.
    let args: &str = unwrap_utf8(&buf[..w]);
    (args, "")
}

/// Converts a byte slice to `&str`, panicking on invalid UTF-8.
fn unwrap_utf8(bytes: &[u8]) -> &str {
    core::str::from_utf8(bytes).expect("cmdline: invalid UTF-8 after compaction")
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: copies the input into a mutable buffer and calls `split_cmdline`.
    fn split(input: &str) -> (::alloc::string::String, ::alloc::string::String) {
        let mut buf: ::alloc::vec::Vec<u8> = input.as_bytes().to_vec();
        let (args, env) = split_cmdline(&mut buf);
        (::alloc::string::String::from(args), ::alloc::string::String::from(env))
    }

    #[test]
    fn args_only() {
        let (args, env) = split("arg1 arg2");
        assert_eq!(args, "arg1 arg2", "args mismatch");
        assert_eq!(env, "", "env should be empty");
    }

    #[test]
    fn args_and_env() {
        let (args, env) = split("arg1 arg2;VAR1=foo VAR2=bar");
        assert_eq!(args, "arg1 arg2", "args mismatch");
        assert_eq!(env, "VAR1=foo VAR2=bar", "env mismatch");
    }

    #[test]
    fn env_only() {
        let (args, env) = split(";VAR1=foo");
        assert_eq!(args, "", "args should be empty");
        assert_eq!(env, "VAR1=foo", "env mismatch");
    }

    #[test]
    fn empty_string() {
        let (args, env) = split("");
        assert_eq!(args, "", "args should be empty");
        assert_eq!(env, "", "env should be empty");
    }

    #[test]
    fn escaped_semicolon_in_args() {
        let (args, env) = split("arg1 with\\;semicolon arg2;VAR1=foo");
        assert_eq!(args, "arg1 with;semicolon arg2", "escaped \\; should become ;");
        assert_eq!(env, "VAR1=foo", "env mismatch");
    }

    #[test]
    fn escaped_semicolon_in_env() {
        let (args, env) = split("arg1;VAR=a\\;b");
        assert_eq!(args, "arg1", "args mismatch");
        assert_eq!(env, "VAR=a;b", "escaped \\; in env should become ;");
    }

    #[test]
    fn multiple_escaped_semicolons() {
        let (args, env) = split("a\\;b\\;c;VAR=d\\;e");
        assert_eq!(args, "a;b;c", "multiple escaped semicolons");
        assert_eq!(env, "VAR=d;e", "env escaped semicolon");
    }

    #[test]
    fn escaped_only_no_separator() {
        let (args, env) = split("a\\;b");
        assert_eq!(args, "a;b", "escaped semicolon without separator");
        assert_eq!(env, "", "env should be empty");
    }

    #[test]
    fn backslash_before_non_semicolon() {
        let (args, env) = split("C:\\path\\file;VAR=x");
        assert_eq!(args, "C:\\path\\file", "lone backslashes preserved");
        assert_eq!(env, "VAR=x", "env mismatch");
    }

    #[test]
    fn backslash_at_end() {
        let (args, env) = split("trail\\");
        assert_eq!(args, "trail\\", "trailing backslash preserved");
        assert_eq!(env, "", "env should be empty");
    }

    #[test]
    fn backslash_semicolon_then_separator() {
        let (args, env) = split("a\\;;VAR=x");
        assert_eq!(args, "a;", "backslash-semicolon then separator");
        assert_eq!(env, "VAR=x", "env after separator");
    }

    #[test]
    fn backward_compatible_no_semicolons() {
        let (args, env) = split("hello world");
        assert_eq!(args, "hello world", "plain args unchanged");
        assert_eq!(env, "", "env should be empty");
    }

    #[test]
    fn backward_compatible_with_separator() {
        let (args, env) = split("prog;HOME=/tmp");
        assert_eq!(args, "prog", "args before separator");
        assert_eq!(env, "HOME=/tmp", "env after separator");
    }
}
