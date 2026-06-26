// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    regcomp::regcomp,
    regerror::regerror,
    regexec::regexec,
    regfree::regfree,
    types::{
        regex_t,
        regmatch_t,
        REG_EESCAPE,
        REG_EXTENDED,
        REG_ICASE,
        REG_MINIMAL,
        REG_NEWLINE,
        REG_NOERROR,
        REG_NOMATCH,
        REG_NOTBOL,
        REG_NOTEOL,
    },
};
use ::std::{
    ffi::CString,
    vec::Vec,
};
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Helpers
//==================================================================================================

/// Returns a freshly zeroed `regex_t`.
fn blank() -> regex_t {
    regex_t {
        re_nsub: 0,
        priv_: core::ptr::null_mut(),
        cflags: 0,
    }
}

/// Compiles `pattern` with `cflags`, returning the compiled expression and the status code.
fn compile(pattern: &str, cflags: c_int) -> (regex_t, c_int) {
    let mut re: regex_t = blank();
    let cs: CString = CString::new(pattern).expect("pattern has no interior NUL");
    let rc: c_int = unsafe { regcomp(&mut re, cs.as_ptr(), cflags) };
    (re, rc)
}

/// Executes `re` against `string`, capturing up to `nmatch` offsets.
fn run(re: &regex_t, string: &str, nmatch: usize, eflags: c_int) -> (c_int, Vec<(isize, isize)>) {
    let cs: CString = CString::new(string).expect("string has no interior NUL");
    let mut pm: Vec<regmatch_t> = ::std::vec![regmatch_t { rm_so: -2, rm_eo: -2 }; nmatch.max(1)];
    let rc: c_int = unsafe { regexec(re, cs.as_ptr(), nmatch, pm.as_mut_ptr(), eflags) };
    let offs: Vec<(isize, isize)> = pm.iter().map(|m| (m.rm_so, m.rm_eo)).collect();
    (rc, offs)
}

/// Convenience: compile + match in one shot, returning the whole-match offsets.
fn whole_match(pattern: &str, cflags: c_int, string: &str) -> Option<(isize, isize)> {
    let (mut re, rc) = compile(pattern, cflags);
    assert_eq!(rc, REG_NOERROR, "compile failed for {pattern:?}");
    let (rc, offs) = run(&re, string, 1, 0);
    unsafe { regfree(&mut re) };
    if rc == REG_NOERROR {
        Some(offs[0])
    } else {
        None
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[test]
fn literal_bre() {
    assert_eq!(whole_match("abc", 0, "xabcy"), Some((1, 4)));
    assert_eq!(whole_match("abc", 0, "xyz"), None);
}

#[test]
fn anchors() {
    assert_eq!(whole_match("^abc", 0, "abcd"), Some((0, 3)));
    assert_eq!(whole_match("^abc", 0, "xabc"), None);
    assert_eq!(whole_match("abc$", 0, "xabc"), Some((1, 4)));
    assert_eq!(whole_match("abc$", 0, "abcx"), None);
}

#[test]
fn dot_any() {
    assert_eq!(whole_match("a.c", 0, "axc"), Some((0, 3)));
    assert_eq!(whole_match("a.c", 0, "ac"), None);
}

#[test]
fn star_and_plus() {
    assert_eq!(whole_match("ab*c", 0, "ac"), Some((0, 2)));
    assert_eq!(whole_match("ab*c", 0, "abbbc"), Some((0, 5)));
    // '+' is special only in ERE.
    assert_eq!(whole_match("ab+c", REG_EXTENDED, "ac"), None);
    assert_eq!(whole_match("ab+c", REG_EXTENDED, "abc"), Some((0, 3)));
}

#[test]
fn alternation_ere() {
    assert_eq!(whole_match("cat|dog", REG_EXTENDED, "hotdog"), Some((3, 6)));
    assert_eq!(whole_match("cat|dog", REG_EXTENDED, "bird"), None);
}

#[test]
fn bre_alternation_and_groups() {
    // BRE uses \| and \( \).
    assert_eq!(whole_match("\\(ab\\)*", 0, "ababx"), Some((0, 4)));
    assert_eq!(whole_match("a\\|b", 0, "b"), Some((0, 1)));
}

#[test]
fn capture_groups() {
    let (mut re, rc) = compile("(a+)(b+)", REG_EXTENDED);
    assert_eq!(rc, REG_NOERROR);
    assert_eq!(re.re_nsub, 2);
    let (rc, offs) = run(&re, "aabbb", 3, 0);
    unsafe { regfree(&mut re) };
    assert_eq!(rc, REG_NOERROR);
    assert_eq!(offs[0], (0, 5));
    assert_eq!(offs[1], (0, 2));
    assert_eq!(offs[2], (2, 5));
}

#[test]
fn character_classes() {
    assert_eq!(whole_match("[0-9][0-9]*", 0, "abc123"), Some((3, 6)));
    assert_eq!(whole_match("[[:digit:]][[:digit:]]*", 0, "x42y"), Some((1, 3)));
    assert_eq!(whole_match("[^0-9]*", 0, "ab9"), Some((0, 2)));
}

#[test]
fn shorthand_classes() {
    assert_eq!(whole_match("\\w\\w*", REG_EXTENDED, " foo "), Some((1, 4)));
    assert_eq!(whole_match("\\d\\d*", REG_EXTENDED, "a12"), Some((1, 3)));
}

#[test]
fn intervals() {
    assert_eq!(whole_match("a{2,3}", REG_EXTENDED, "aaaa"), Some((0, 3)));
    assert_eq!(whole_match("a{2}", REG_EXTENDED, "aaaa"), Some((0, 2)));
    assert_eq!(whole_match("a{2,}", REG_EXTENDED, "aaaa"), Some((0, 4)));
    assert_eq!(whole_match("ba{2,3}", REG_EXTENDED, "ba"), None);
}

#[test]
fn case_insensitive() {
    assert_eq!(whole_match("abc", REG_ICASE, "xABCy"), Some((1, 4)));
    assert_eq!(whole_match("[a-z]*", REG_ICASE, "ABC"), Some((0, 3)));
}

#[test]
fn newline_sensitivity() {
    // Under REG_NEWLINE, '.' (and ANY) no longer match a newline.
    assert_eq!(whole_match("a.b", REG_EXTENDED, "a\nb"), Some((0, 3)));
    assert_eq!(whole_match("a.b", REG_EXTENDED | REG_NEWLINE, "a\nb"), None);
    // '$' matches just before an embedded newline under REG_NEWLINE.
    assert_eq!(whole_match("a$", REG_NEWLINE, "a\nb"), Some((0, 1)));
    assert_eq!(whole_match("a$", 0, "a\nb"), None);

    // The unanchored search must still advance past embedded newlines under REG_NEWLINE, so a
    // pattern can match on a line after the first.
    assert_eq!(whole_match("b", REG_NEWLINE, "a\nb"), Some((2, 3)));
    assert_eq!(whole_match("b", REG_EXTENDED | REG_NEWLINE, "a\nb"), Some((2, 3)));

    // A positive bracket expression still matches '\n' when it is explicitly included.
    assert_eq!(whole_match("a[\n]b", REG_EXTENDED | REG_NEWLINE, "a\nb"), Some((0, 3)));
    assert_eq!(whole_match("[\n]", REG_NEWLINE, "a\nb"), Some((1, 2)));
    // A complemented bracket expression does not match '\n' under REG_NEWLINE, even though it
    // would otherwise (without the flag).
    assert_eq!(whole_match("a[^x]b", REG_EXTENDED | REG_NEWLINE, "a\nb"), None);
    assert_eq!(whole_match("a[^x]b", REG_EXTENDED, "a\nb"), Some((0, 3)));
}

#[test]
fn notbol_noteol() {
    let (mut re, rc) = compile("^a", 0);
    assert_eq!(rc, REG_NOERROR);
    let (rc_plain, _) = run(&re, "abc", 1, 0);
    let (rc_notbol, _) = run(&re, "abc", 1, REG_NOTBOL);
    unsafe { regfree(&mut re) };
    assert_eq!(rc_plain, REG_NOERROR);
    assert_eq!(rc_notbol, REG_NOMATCH);

    let (mut re, rc) = compile("c$", 0);
    assert_eq!(rc, REG_NOERROR);
    let (rc_plain, _) = run(&re, "abc", 1, 0);
    let (rc_noteol, _) = run(&re, "abc", 1, REG_NOTEOL);
    unsafe { regfree(&mut re) };
    assert_eq!(rc_plain, REG_NOERROR);
    assert_eq!(rc_noteol, REG_NOMATCH);
}

#[test]
fn leftmost_longest_like() {
    // Leftmost start wins; the greedy star then extends as far as possible.
    assert_eq!(whole_match("a.*b", REG_EXTENDED, "xaybzbq"), Some((1, 6)));
}

#[test]
fn leftmost_longest_alternation() {
    assert_eq!(whole_match("a|ab", REG_EXTENDED, "ab"), Some((0, 2)));
    // The shorter first-group alternative yields the longest overall match.
    assert_eq!(whole_match("(ab|a)(c|bcd)", REG_EXTENDED, "abcd"), Some((0, 4)));
}

#[test]
fn minimal_repetition() {
    assert_eq!(whole_match("a.*b", REG_EXTENDED | REG_MINIMAL, "axbyb"), Some((0, 3)));
    assert_eq!(whole_match("a.*?b", REG_EXTENDED, "axbyb"), Some((0, 3)));
    assert_eq!(whole_match("a.*?b.*c", REG_EXTENDED, "axbycxxc"), Some((0, 8)));
}

#[test]
fn compile_errors() {
    let (_, rc) = compile("(", REG_EXTENDED);
    assert_ne!(rc, REG_NOERROR);
    let (_, rc) = compile("[abc", REG_EXTENDED);
    assert_ne!(rc, REG_NOERROR);
    let (_, rc) = compile("a{3,2}", REG_EXTENDED);
    assert_ne!(rc, REG_NOERROR);
    let (_, rc) = compile("abc\\", REG_EXTENDED);
    assert_eq!(rc, REG_EESCAPE);
}

#[test]
fn regerror_message() {
    let mut buf: [c_char; 32] = [0; 32];
    let need: usize =
        unsafe { regerror(REG_NOMATCH, core::ptr::null(), buf.as_mut_ptr(), buf.len()) };
    assert_eq!(need, b"no match".len() + 1);
    let bytes: Vec<u8> = buf
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect();
    assert_eq!(&bytes, b"no match");
}

#[test]
fn regerror_truncation() {
    let mut buf: [c_char; 4] = [0; 4];
    let need: usize =
        unsafe { regerror(REG_BADPAT_CODE, core::ptr::null(), buf.as_mut_ptr(), buf.len()) };
    // The needed size reflects the full message, even though the buffer is small.
    assert!(need > 4);
    // The buffer is NUL-terminated within its bounds.
    assert_eq!(buf[3], 0);
}

/// Local alias so the truncation test does not need to import the constant by a long path.
const REG_BADPAT_CODE: c_int = crate::types::REG_BADPAT;
