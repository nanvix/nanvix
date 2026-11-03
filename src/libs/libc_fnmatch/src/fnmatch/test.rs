// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    fnmatch,
    FNM_CASEFOLD,
    FNM_NOESCAPE,
    FNM_PATHNAME,
    FNM_PERIOD,
};
use ::std::ffi::CString;
use ::sysapi::ffi::c_int;

/// Returns the result of matching `string` against `pattern` with `flags`.
fn matches(pattern: &str, string: &str, flags: c_int) -> bool {
    let p: CString = CString::new(pattern).expect("no interior nul");
    let s: CString = CString::new(string).expect("no interior nul");
    unsafe { fnmatch(p.as_ptr(), s.as_ptr(), flags) == 0 }
}

#[test]
fn literal_and_question_and_star() {
    assert!(matches("abc", "abc", 0));
    assert!(!matches("abc", "abd", 0));
    assert!(matches("a?c", "abc", 0));
    assert!(!matches("a?c", "ac", 0));
    assert!(matches("a*c", "abbbc", 0));
    assert!(matches("a*c", "ac", 0));
    assert!(matches("*", "anything", 0));
    assert!(matches("**", "anything", 0));
}

#[test]
fn bracket_expressions() {
    assert!(matches("[abc]", "b", 0));
    assert!(!matches("[abc]", "d", 0));
    assert!(matches("[a-z]", "m", 0));
    assert!(!matches("[a-z]", "M", 0));
    assert!(matches("[!a-z]", "M", 0));
    assert!(matches("[^a-z]", "0", 0));
    assert!(matches("file[0-9].txt", "file7.txt", 0));
}

#[test]
fn pathname_flag_blocks_slash() {
    assert!(!matches("a*c", "a/c", FNM_PATHNAME));
    assert!(!matches("a?c", "a/c", FNM_PATHNAME));
    assert!(matches("a/c", "a/c", FNM_PATHNAME));
    assert!(matches("*/c", "a/c", FNM_PATHNAME));
    assert!(!matches("*", "a/c", FNM_PATHNAME));
}

#[test]
fn period_flag_blocks_leading_dot() {
    assert!(!matches("*", ".hidden", FNM_PERIOD));
    assert!(matches(".*", ".hidden", FNM_PERIOD));
    assert!(!matches("?hidden", ".hidden", FNM_PERIOD));
    // Without FNM_PERIOD a wildcard matches a leading dot.
    assert!(matches("*", ".hidden", 0));
}

#[test]
fn period_after_slash_with_pathname() {
    assert!(!matches("a/*", "a/.b", FNM_PATHNAME | FNM_PERIOD));
    assert!(matches("a/.*", "a/.b", FNM_PATHNAME | FNM_PERIOD));
}

#[test]
fn period_flag_blocks_leading_dot_in_bracket() {
    // A bracket expression is a wildcard, so under FNM_PERIOD it must not match a
    // leading '.'.
    assert!(!matches("[!a]*", ".hidden", FNM_PERIOD));
    // Without FNM_PERIOD a bracket expression matches a leading dot.
    assert!(matches("[!a]*", ".hidden", 0));
    // The same rule applies to the byte after a '/' under FNM_PATHNAME.
    assert!(!matches("a/[!a]*", "a/.b", FNM_PATHNAME | FNM_PERIOD));
    assert!(matches("a/[!a]*", "a/b", FNM_PATHNAME | FNM_PERIOD));
}

#[test]
fn noescape_flag() {
    // With escaping (default), "\*" matches a literal '*'.
    assert!(matches("\\*", "*", 0));
    assert!(!matches("\\*", "a", 0));
    // With FNM_NOESCAPE, the backslash is itself a literal character.
    assert!(matches("\\", "\\", FNM_NOESCAPE));
    assert!(!matches("\\", "a", FNM_NOESCAPE));
}

#[test]
fn bracket_leading_bracket_is_literal() {
    // A ']' as the first member denotes a literal ']'.
    assert!(matches("[]]", "]", 0));
    assert!(matches("[]a]", "a", 0));
    assert!(matches("[!]]", "a", 0));
    assert!(!matches("[!]]", "]", 0));
}

#[test]
fn invalid_bracket_matches_literal() {
    // A '[' that does not introduce a valid bracket expression matches itself.
    assert!(matches("[", "[", 0));
    assert!(matches("[abc", "[abc", 0));
    assert!(matches("a[b", "a[b", 0));
    assert!(!matches("[abc", "a", 0));
}

#[test]
fn bracket_character_classes() {
    assert!(matches("[[:digit:]]", "7", 0));
    assert!(!matches("[[:digit:]]", "a", 0));
    assert!(matches("[[:alpha:]]", "Q", 0));
    assert!(matches("file[[:digit:]].txt", "file3.txt", 0));
    assert!(matches("x[[:space:]]y", "x y", 0));
    assert!(matches("[![:digit:]]", "a", 0));
    assert!(!matches("[![:digit:]]", "5", 0));
}

#[test]
fn bracket_collating_and_equivalence() {
    // In the C locale these denote a single character.
    assert!(matches("[[=a=]]", "a", 0));
    assert!(!matches("[[=a=]]", "b", 0));
    assert!(matches("[[.b.]]", "b", 0));
}

#[test]
fn casefold_flag() {
    assert!(matches("ABC", "abc", FNM_CASEFOLD));
    assert!(matches("abc", "ABC", FNM_CASEFOLD));
    assert!(!matches("abc", "abd", FNM_CASEFOLD));
    assert!(matches("[a-z]", "M", FNM_CASEFOLD));
    assert!(matches("[A-Z]", "m", FNM_CASEFOLD));
    assert!(matches("\\A", "a", FNM_CASEFOLD));
    assert!(matches("[[:upper:]]", "a", FNM_CASEFOLD));
    assert!(matches("[[:lower:]]", "A", FNM_CASEFOLD));
    // Without the flag, matching is case-sensitive.
    assert!(!matches("ABC", "abc", 0));
}

#[test]
fn star_period_after_slash_past_wildcard() {
    // A '*' before a '/' must not let the component after the '/' bypass the
    // leading-period rule (regression test for FNM_PERIOD across '*').
    assert!(!matches("*/*", "a/.b", FNM_PATHNAME | FNM_PERIOD));
    assert!(matches("*/.*", "a/.b", FNM_PATHNAME | FNM_PERIOD));
    assert!(matches("*/*", "a/b", FNM_PATHNAME | FNM_PERIOD));
}
