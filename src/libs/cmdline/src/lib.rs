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
mod tests;
