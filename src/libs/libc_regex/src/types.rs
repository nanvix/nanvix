// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_int,
    c_void,
};

//==================================================================================================
// Types
//==================================================================================================

/// Signed byte offset within a string, as used by `regmatch_t`.
///
/// POSIX requires `regoff_t` to be a signed integer type wide enough to hold the largest value
/// representable by `ptrdiff_t`/`ssize_t`. It is therefore defined as `isize`, matching the
/// `ptrdiff_t`-based typedef emitted in the generated `regex.h`.
pub type regoff_t = isize;

/// Compiled regular expression.
///
/// Only `re_nsub` is part of the public POSIX contract; the remaining fields are private to this
/// implementation. The layout matches the `regex_t` declared in the generated `regex.h`.
#[repr(C)]
pub struct regex_t {
    /// Number of parenthesized subexpressions in the pattern.
    pub re_nsub: usize,
    /// Opaque pointer to the compiled program (a boxed [`crate::prog::Prog`]).
    pub priv_: *mut c_void,
    /// Compile flags captured at `regcomp()` time.
    pub cflags: c_int,
}

/// Match offsets for a single (sub)expression.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct regmatch_t {
    /// Byte offset of the start of the match, or `-1` if unused.
    pub rm_so: regoff_t,
    /// Byte offset just past the end of the match, or `-1` if unused.
    pub rm_eo: regoff_t,
}

//==================================================================================================
// Constants
//==================================================================================================

// Compile flags (cflags) for `regcomp()`.

/// Use Extended Regular Expression syntax.
pub const REG_EXTENDED: c_int = 0x0001;
/// Ignore case when matching.
pub const REG_ICASE: c_int = 0x0002;
/// Newline-sensitive matching.
pub const REG_NEWLINE: c_int = 0x0004;
/// Report only success/failure; do not record submatches.
pub const REG_NOSUB: c_int = 0x0008;
/// Prefer the leftmost shortest match for ERE duplication symbols.
pub const REG_MINIMAL: c_int = 0x0010;

// Execution flags (eflags) for `regexec()`.

/// The beginning of the string is not the beginning of a line.
pub const REG_NOTBOL: c_int = 0x0100;
/// The end of the string is not the end of a line.
pub const REG_NOTEOL: c_int = 0x0200;

// Result and error codes.

/// Success.
pub const REG_NOERROR: c_int = 0;
/// The pattern did not match.
pub const REG_NOMATCH: c_int = 1;
/// Invalid regular expression.
pub const REG_BADPAT: c_int = 2;
/// Invalid collating element.
pub const REG_ECOLLATE: c_int = 3;
/// Invalid character class name.
pub const REG_ECTYPE: c_int = 4;
/// Trailing backslash.
pub const REG_EESCAPE: c_int = 5;
/// Invalid back reference.
pub const REG_ESUBREG: c_int = 6;
/// Unbalanced `[` `]`.
pub const REG_EBRACK: c_int = 7;
/// Unbalanced `(` `)`.
pub const REG_EPAREN: c_int = 8;
/// Unbalanced `{` `}`.
pub const REG_EBRACE: c_int = 9;
/// Invalid content of `{` `}`.
pub const REG_BADBR: c_int = 10;
/// Invalid range end.
pub const REG_ERANGE: c_int = 11;
/// Out of memory.
pub const REG_ESPACE: c_int = 12;
/// `?`, `*`, or `+` not preceded by a valid expression.
pub const REG_BADRPT: c_int = 13;
