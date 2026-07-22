// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_char,
    c_int,
    c_uchar,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Return value of [`fnmatch`] when `string` does not match `pattern`.
pub const FNM_NOMATCH: c_int = 1;

/// Flag: a wildcard (`*`, `?`, or `[...]`) never matches a `'/'`.
pub const FNM_PATHNAME: c_int = 1 << 0;
/// Flag: a backslash does not quote the following character.
pub const FNM_NOESCAPE: c_int = 1 << 1;
/// Flag: a leading `'.'` in `string` is matched only by an explicit `'.'` in `pattern`.
pub const FNM_PERIOD: c_int = 1 << 2;
/// Flag: ASCII letters in `string` and `pattern` are compared without regard to case.
pub const FNM_CASEFOLD: c_int = 1 << 4;
/// Alias for [`FNM_CASEFOLD`].
pub const FNM_IGNORECASE: c_int = FNM_CASEFOLD;

//==================================================================================================
// Helpers
//==================================================================================================

/// Returns `true` if the null-terminated string `s` contains a `'/'`.
///
/// # Safety
///
/// `s` must point to a valid null-terminated C string.
unsafe fn contains_slash(mut s: *const c_uchar) -> bool {
    unsafe {
        while *s != 0 {
            if *s == b'/' {
                return true;
            }
            s = s.add(1);
        }
        false
    }
}

/// Returns `true` if `sc` is a leading period that `FNM_PERIOD` forbids a wildcard from matching.
///
/// A period is "leading" when it is the first byte of `string`, or (under `FNM_PATHNAME`) the byte
/// immediately after a `'/'`.
///
/// # Safety
///
/// `s` must point within the same allocation as `s_start`, and `s.sub(1)` must be dereferenceable
/// whenever `s != s_start`.
unsafe fn is_hidden_period(
    sc: c_uchar,
    s: *const c_uchar,
    s_start: *const c_uchar,
    flags: c_int,
) -> bool {
    unsafe {
        (flags & FNM_PERIOD) != 0
            && sc == b'.'
            && (s == s_start || ((flags & FNM_PATHNAME) != 0 && *s.sub(1) == b'/'))
    }
}

/// Folds an ASCII byte to lowercase when `FNM_CASEFOLD` is in effect; otherwise returns it
/// unchanged.
fn fold(c: c_uchar, flags: c_int) -> c_uchar {
    if (flags & FNM_CASEFOLD) != 0 {
        c.to_ascii_lowercase()
    } else {
        c
    }
}

/// Returns `true` if byte `c` belongs to the POSIX character class `name`, evaluated in the C
/// locale. Unknown class names never match.
fn char_class_match(name: &[u8], c: c_uchar, flags: c_int) -> bool {
    match name {
        b"alnum" => c.is_ascii_alphanumeric(),
        b"alpha" => c.is_ascii_alphabetic(),
        b"blank" => c == b' ' || c == b'\t',
        b"cntrl" => c.is_ascii_control(),
        b"digit" => c.is_ascii_digit(),
        b"graph" => c.is_ascii_graphic(),
        b"lower" => {
            c.is_ascii_lowercase() || ((flags & FNM_CASEFOLD) != 0 && c.is_ascii_uppercase())
        },
        b"print" => c == b' ' || c.is_ascii_graphic(),
        b"punct" => c.is_ascii_punctuation(),
        b"space" => c == b' ' || (b'\t'..=b'\r').contains(&c),
        b"upper" => {
            c.is_ascii_uppercase() || ((flags & FNM_CASEFOLD) != 0 && c.is_ascii_lowercase())
        },
        b"xdigit" => c.is_ascii_hexdigit(),
        _ => false,
    }
}

/// Evaluates the bracket expression whose body begins at `body` (the byte after the opening
/// `'['`) against the character `sc`.
///
/// On success returns `(matched, rest)`, where `matched` reports whether `sc` is matched and
/// `rest` points just past the closing `']'`. Returns [`None`] when the bytes do not form a
/// valid, closed bracket expression; in that case the caller treats the `'['` as a literal
/// character, as required by POSIX.
///
/// Supports `'!'`/`'^'` negation, `a-z` ranges, POSIX character classes (for example
/// `[[:alpha:]]`), and—in the C locale—single-character collating (`[[.x.]]`) and equivalence
/// (`[[=x=]]`) elements.
///
/// # Safety
///
/// `body` must point into a valid null-terminated C string.
unsafe fn match_bracket(
    body: *const c_uchar,
    sc: c_uchar,
    flags: c_int,
) -> Option<(bool, *const c_uchar)> {
    unsafe {
        let mut p: *const c_uchar = body;

        // A leading `'!'` (or, as a widespread extension, `'^'`) negates the set.
        let negate: bool = *p == b'!' || *p == b'^';
        if negate {
            p = p.add(1);
        }

        let folded_sc: c_uchar = fold(sc, flags);
        let mut matched: bool = false;
        let mut first: bool = true;

        loop {
            let c: c_uchar = *p;

            // Reaching the end of the pattern before a closing `']'` means this is not a valid
            // bracket expression.
            if c == 0 {
                return None;
            }

            // A `']'` closes the expression, unless it is the first member (where it denotes a
            // literal `']'`).
            if c == b']' && !first {
                return Some((negate != matched, p.add(1)));
            }
            first = false;

            // A class (`[:name:]`), collating (`[.x.]`), or equivalence (`[=x=]`) sub-expression.
            if c == b'[' && matches!(*p.add(1), b':' | b'.' | b'=') {
                let kind: c_uchar = *p.add(1);
                let content: *const c_uchar = p.add(2);
                let mut q: *const c_uchar = content;
                loop {
                    let qc: c_uchar = *q;
                    if qc == 0 {
                        // Unterminated sub-expression: not a valid bracket expression.
                        return None;
                    }
                    if qc == kind && *q.add(1) == b']' {
                        break;
                    }
                    q = q.add(1);
                }
                if kind == b':' {
                    let len: usize = q.offset_from(content).unsigned_abs();
                    let name: &[u8] = ::core::slice::from_raw_parts(content, len);
                    if char_class_match(name, sc, flags) {
                        matched = true;
                    }
                } else if q == content.add(1) && fold(*content, flags) == folded_sc {
                    // A single-character collating element or equivalence class.
                    matched = true;
                }
                p = q.add(2);
                continue;
            }

            // An ordinary member, possibly the low end of a range `c-d`. A `'-'` denotes a range
            // only when it is neither the first nor the last member.
            if *p.add(1) == b'-' && *p.add(2) != b']' && *p.add(2) != 0 {
                let low: c_uchar = fold(c, flags);
                let high: c_uchar = fold(*p.add(2), flags);
                if folded_sc >= low && folded_sc <= high {
                    matched = true;
                }
                p = p.add(3);
            } else {
                if fold(c, flags) == folded_sc {
                    matched = true;
                }
                p = p.add(1);
            }
        }
    }
}

/// Recursively matches `pattern` against `string`.
///
/// `s_start` is the original start of `string`, retained so that `FNM_PERIOD` can recognize a
/// leading period across recursive calls.
///
/// # Safety
///
/// `p`, `s`, and `s_start` must point to valid null-terminated C strings within the same string
/// allocation for `s`/`s_start`.
unsafe fn fnmatch_internal(
    mut p: *const c_uchar,
    mut s: *const c_uchar,
    s_start: *const c_uchar,
    flags: c_int,
) -> c_int {
    unsafe {
        loop {
            let c: c_uchar = *p;
            p = p.add(1);
            if c == 0 {
                break;
            }
            let sc: c_uchar = *s;

            match c {
                b'?' => {
                    if sc == 0 {
                        return FNM_NOMATCH;
                    }
                    if (flags & FNM_PATHNAME) != 0 && sc == b'/' {
                        return FNM_NOMATCH;
                    }
                    if is_hidden_period(sc, s, s_start, flags) {
                        return FNM_NOMATCH;
                    }
                    s = s.add(1);
                },

                b'*' => {
                    // Collapse a run of consecutive stars.
                    while *p == b'*' {
                        p = p.add(1);
                    }

                    if is_hidden_period(sc, s, s_start, flags) {
                        return FNM_NOMATCH;
                    }

                    // A trailing star matches the rest of the component (or string).
                    if *p == 0 {
                        if (flags & FNM_PATHNAME) != 0 {
                            return if contains_slash(s) { FNM_NOMATCH } else { 0 };
                        }
                        return 0;
                    }

                    // Try to match the remainder of the pattern at each position. The
                    // leading-period rule is re-evaluated by position (see `is_hidden_period`),
                    // so `flags` is forwarded unchanged; in particular a `'.'` immediately after
                    // a `'/'` under `FNM_PATHNAME | FNM_PERIOD` remains protected even when it is
                    // reached past this `'*'`.
                    let mut t: *const c_uchar = s;
                    while *t != 0 {
                        if (flags & FNM_PATHNAME) != 0 && *t == b'/' {
                            break;
                        }
                        if fnmatch_internal(p, t, s_start, flags) == 0 {
                            return 0;
                        }
                        t = t.add(1);
                    }
                    return fnmatch_internal(p, t, s_start, flags);
                },

                b'[' => {
                    match match_bracket(p, sc, flags) {
                        Some((member_matched, rest)) => {
                            // A valid bracket expression behaves as a single-character wildcard,
                            // so it is subject to the same `FNM_PATHNAME` and `FNM_PERIOD`
                            // restrictions as `'?'`.
                            if sc == 0 {
                                return FNM_NOMATCH;
                            }
                            if (flags & FNM_PATHNAME) != 0 && sc == b'/' {
                                return FNM_NOMATCH;
                            }
                            if is_hidden_period(sc, s, s_start, flags) {
                                return FNM_NOMATCH;
                            }
                            if !member_matched {
                                return FNM_NOMATCH;
                            }
                            p = rest;
                            s = s.add(1);
                        },
                        None => {
                            // The bytes do not form a valid bracket expression, so the `'['`
                            // matches itself literally.
                            if fold(b'[', flags) != fold(sc, flags) {
                                return FNM_NOMATCH;
                            }
                            s = s.add(1);
                        },
                    }
                },

                _ => {
                    // Literal byte, possibly backslash-escaped.
                    let literal: c_uchar = if c == b'\\' && (flags & FNM_NOESCAPE) == 0 {
                        let escaped: c_uchar = *p;
                        p = p.add(1);
                        if escaped == 0 {
                            return FNM_NOMATCH;
                        }
                        escaped
                    } else {
                        c
                    };
                    if fold(literal, flags) != fold(sc, flags) {
                        return FNM_NOMATCH;
                    }
                    s = s.add(1);
                },
            }
        }

        if *s == 0 {
            0
        } else {
            FNM_NOMATCH
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Matches the null-terminated string `string` against the shell wildcard pattern `pattern`.
///
/// The pattern language supports `?` (any single character), `*` (any sequence), and `[...]`
/// bracket expressions, including `!`/`^` negation, `a-z` ranges, POSIX character classes such as
/// `[[:digit:]]`, and (in the C locale) collating `[[.x.]]` and equivalence `[[=x=]]` elements. A
/// `[` that does not introduce a valid bracket expression matches itself. Behavior is adjusted by
/// `flags`: `FNM_PATHNAME` prevents wildcards from matching `'/'`, `FNM_PERIOD` requires a leading
/// `'.'` to be matched explicitly, `FNM_NOESCAPE` disables backslash escaping, and `FNM_CASEFOLD`
/// compares ASCII letters case-insensitively.
///
/// # Parameters
///
/// - `pattern`: The wildcard pattern.
/// - `string`: The string to match against `pattern`.
/// - `flags`: A bitwise OR of the `FNM_*` flags.
///
/// # Returns
///
/// `0` if `string` matches `pattern`, or `FNM_NOMATCH` (1) otherwise.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `pattern` and `string`, which
/// must reference valid null-terminated C strings.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fnmatch.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fnmatch(
    pattern: *const c_char,
    string: *const c_char,
    flags: c_int,
) -> c_int {
    unsafe {
        let p: *const c_uchar = pattern.cast::<c_uchar>();
        let s: *const c_uchar = string.cast::<c_uchar>();
        fnmatch_internal(p, s, s, flags)
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
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
}
