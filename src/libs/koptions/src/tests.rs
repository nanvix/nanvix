// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
