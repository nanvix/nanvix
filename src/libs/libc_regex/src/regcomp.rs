// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    parser::Parser,
    prog::{
        build_prog,
        Prog,
    },
    types::{
        regex_t,
        REG_BADPAT,
        REG_EXTENDED,
        REG_ICASE,
        REG_MINIMAL,
        REG_NEWLINE,
        REG_NOERROR,
    },
};
use ::sysapi::ffi::{
    c_char,
    c_int,
    c_void,
};
use alloc::boxed::Box;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Compiles the regular expression `pattern` into `preg` according to `cflags`.
///
/// # Parameters
///
/// - `preg`: Output compiled-expression structure.
/// - `pattern`: NUL-terminated regular expression to compile.
/// - `cflags`: Bitwise OR of `REG_EXTENDED`, `REG_ICASE`, `REG_MINIMAL`, `REG_NEWLINE`, and
///   `REG_NOSUB`.
///
/// # Returns
///
/// Returns `REG_NOERROR` (`0`) on success, or a non-zero `REG_*` error code on failure.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `preg` and `pattern`. The
/// caller must ensure that `preg` points to a writable `regex_t` and that `pattern` points to a
/// valid NUL-terminated string.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn regcomp(
    preg: *mut regex_t,
    pattern: *const c_char,
    cflags: c_int,
) -> c_int {
    if preg.is_null() || pattern.is_null() {
        return REG_BADPAT;
    }

    let bytes: &[u8] = core::ffi::CStr::from_ptr(pattern).to_bytes();
    let ere: bool = cflags & REG_EXTENDED != 0;
    let icase: bool = cflags & REG_ICASE != 0;
    let newline: bool = cflags & REG_NEWLINE != 0;
    let minimal: bool = ere && cflags & REG_MINIMAL != 0;

    let mut parser: Parser = Parser::new(bytes, ere, newline, minimal);
    let tree = parser.parse_alt();
    if parser.err != 0 || !parser.at_end() {
        return if parser.err != 0 {
            parser.err
        } else {
            REG_BADPAT
        };
    }
    let tree = match tree {
        Some(tree) => tree,
        None => return REG_BADPAT,
    };

    let prog: Prog = build_prog(&tree, parser.ngroup, icase, newline);
    let boxed: Box<Prog> = Box::new(prog);

    (*preg).re_nsub = usize::try_from(parser.ngroup).unwrap_or(0);
    (*preg).priv_ = Box::into_raw(boxed).cast::<c_void>();
    (*preg).cflags = cflags;
    REG_NOERROR
}
