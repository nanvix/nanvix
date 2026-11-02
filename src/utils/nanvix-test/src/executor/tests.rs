// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn workload_spec_expected_exit_code_some() {
    let spec: WorkloadSpec =
        WorkloadSpec::new("./bin/test.elf", None, None, None, None, true, Some(0));
    assert_eq!(spec.expected_exit_code(), 0);
}

#[test]
fn workload_spec_expected_exit_code_none() {
    let spec: WorkloadSpec =
        WorkloadSpec::new("./bin/test.elf", None, None, None, None, false, None);
    assert_eq!(spec.expected_exit_code(), 0);
}

#[test]
fn workload_spec_expected_exit_code_nonzero() {
    let spec: WorkloadSpec =
        WorkloadSpec::new("./bin/test.elf", None, None, None, None, true, Some(13));
    assert_eq!(spec.expected_exit_code(), 13);
}

#[test]
fn workload_spec_expected_exit_code_negative() {
    let spec: WorkloadSpec =
        WorkloadSpec::new("./bin/test.elf", None, None, None, None, false, Some(-1));
    assert_eq!(spec.expected_exit_code(), -1);
}

#[test]
fn combine_args_env_no_args_no_env() {
    assert_eq!(combine_args_env(None, None), "");
}

#[test]
fn combine_args_env_args_only() {
    assert_eq!(combine_args_env(Some("arg1 arg2"), None), "arg1 arg2");
}

#[test]
fn combine_args_env_env_only() {
    assert_eq!(combine_args_env(None, Some("VAR=x")), ";VAR=x");
}

#[test]
fn combine_args_env_args_and_env() {
    assert_eq!(combine_args_env(Some("arg1"), Some("VAR=x")), "arg1;VAR=x");
}

#[test]
fn combine_args_env_escapes_semicolons_in_args() {
    assert_eq!(combine_args_env(Some("a;b"), Some("VAR=x")), "a\\;b;VAR=x");
}

#[test]
fn combine_args_env_escapes_semicolons_in_env() {
    assert_eq!(combine_args_env(Some("arg1"), Some("PATH=a;b")), "arg1;PATH=a\\;b");
}

#[test]
fn combine_args_env_escapes_semicolons_even_without_env() {
    assert_eq!(combine_args_env(Some("a;b"), None), "a\\;b");
}

#[test]
fn combine_args_env_empty_env_string_treated_as_absent() {
    assert_eq!(combine_args_env(Some("arg1"), Some("")), "arg1");
}

#[test]
fn combine_args_env_empty_args_string_treated_as_absent() {
    assert_eq!(combine_args_env(Some(""), Some("VAR=x")), ";VAR=x");
}

#[test]
fn combine_args_env_preserves_backslash_semicolon_in_raw_input() {
    // Raw input `\;` (literal backslash + literal semicolon) must be preserved.
    // The `;` is escaped to `\;`, producing `\\;` in the encoded output.
    // split_cmdline() interprets `\\;` as: literal `\` (next char is `\`, not `;`)
    // followed by `\;` escape → `;`. Round-trip result: `\;` — original preserved.
    assert_eq!(combine_args_env(Some("a\\;b"), None), "a\\\\;b");
}
