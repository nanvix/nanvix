// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![deny(clippy::all)]

extern crate alloc;

//==================================================================================================
// Imports
//==================================================================================================

use alloc::vec::Vec;

//==================================================================================================
// Constants
//==================================================================================================

/// Token string recognised as the snapshot kernel option.
pub const SNAPSHOT_TOKEN: &str = "snapshot";

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Represents a parsed kernel option passed via the kernel arguments string.
///
/// Kernel arguments are key=value pairs separated by spaces in the boot command line.
/// Each variant corresponds to a recognised option; unrecognised keys are captured by
/// [`KernelOption::Unknown`] so that no argument is silently dropped.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelOption<'a> {
    /// Enables the guest VM to initiate exactly one VM snapshot.
    /// Format: bare `snapshot` token (no value).
    Snapshot,
    /// An unrecognised kernel argument preserved verbatim for diagnostics.
    /// Format: `<key>=<value>` or a bare `<key>`.
    Unknown(&'a str),
}

///
/// # Description
///
/// Parses the raw kernel arguments string into a list of [`KernelOption`] values.
///
/// The input string is expected to contain space-separated tokens, each of which is either
/// a `key=value` pair or a bare flag.
///
/// # Parameters
///
/// - `kernel_args`: The raw kernel arguments string obtained from boot info.
///
/// # Returns
///
/// A vector of parsed [`KernelOption`] entries. Returns an empty vector when `kernel_args`
/// is empty.
///
#[must_use]
pub fn parse<'a>(kernel_args: &'a str) -> Vec<KernelOption<'a>> {
    let mut options: Vec<KernelOption<'a>> = Vec::new();

    for token in kernel_args.split_whitespace() {
        let option: KernelOption<'a> = match token {
            SNAPSHOT_TOKEN => KernelOption::Snapshot,
            _ => KernelOption::Unknown(token),
        };

        options.push(option);
    }

    options
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Empty input yields no options.
    #[test]
    fn parse_empty() {
        let options: Vec<KernelOption<'_>> = parse("");
        assert!(options.is_empty());
    }

    /// A single key=value token is parsed as Unknown.
    #[test]
    fn parse_key_value() {
        let options: Vec<KernelOption<'_>> = parse("test_magic=0xDEADBEEF");
        assert_eq!(options.len(), 1);
        assert_eq!(options[0], KernelOption::Unknown("test_magic=0xDEADBEEF"));
    }

    /// Unrecognised tokens are captured as Unknown.
    #[test]
    fn parse_unknown() {
        let options: Vec<KernelOption<'_>> = parse("foo=bar");
        assert_eq!(options.len(), 1);
        assert_eq!(options[0], KernelOption::Unknown("foo=bar"));
    }

    /// Multiple space-separated tokens are parsed independently.
    #[test]
    fn parse_multiple() {
        let options: Vec<KernelOption<'_>> = parse("test_magic=0xCAFE unknown_flag");
        assert_eq!(options.len(), 2);
        assert_eq!(options[0], KernelOption::Unknown("test_magic=0xCAFE"));
        assert_eq!(options[1], KernelOption::Unknown("unknown_flag"));
    }

    /// The bare `snapshot` token is parsed as Snapshot.
    #[test]
    fn parse_snapshot() {
        let options: Vec<KernelOption<'_>> = parse("snapshot");
        assert_eq!(options.len(), 1);
        assert_eq!(options[0], KernelOption::Snapshot);
    }

    /// Snapshot mixed with other options.
    #[test]
    fn parse_snapshot_with_others() {
        let options: Vec<KernelOption<'_>> = parse("foo=bar snapshot baz");
        assert_eq!(options.len(), 3);
        assert_eq!(options[0], KernelOption::Unknown("foo=bar"));
        assert_eq!(options[1], KernelOption::Snapshot);
        assert_eq!(options[2], KernelOption::Unknown("baz"));
    }
}
