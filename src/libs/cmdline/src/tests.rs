// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
