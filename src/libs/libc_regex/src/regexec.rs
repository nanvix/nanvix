// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    matcher::exec,
    prog::Prog,
    types::{
        regex_t,
        regmatch_t,
        regoff_t,
        REG_NOERROR,
        REG_NOMATCH,
        REG_NOSUB,
        REG_NOTBOL,
        REG_NOTEOL,
    },
};
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Matches the compiled regular expression `preg` against `string`.
///
/// # Parameters
///
/// - `preg`: Compiled expression produced by `regcomp()`.
/// - `string`: NUL-terminated subject string.
/// - `nmatch`: Number of entries available in `pmatch`.
/// - `pmatch`: Optional array that receives the match and submatch offsets.
/// - `eflags`: Bitwise OR of `REG_NOTBOL` and `REG_NOTEOL`.
///
/// # Returns
///
/// Returns `REG_NOERROR` (`0`) if the pattern matches, or `REG_NOMATCH` otherwise.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `preg`, `string`, and (when
/// non-null) `pmatch`. The caller must ensure that `preg` was initialized by `regcomp()`, that
/// `string` points to a valid NUL-terminated string, and that `pmatch` (if non-null) has room for
/// `nmatch` entries.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn regexec(
    preg: *const regex_t,
    string: *const c_char,
    nmatch: usize,
    pmatch: *mut regmatch_t,
    eflags: c_int,
) -> c_int {
    if preg.is_null() || string.is_null() {
        return REG_NOMATCH;
    }
    let priv_: *mut core::ffi::c_void = (*preg).priv_;
    if priv_.is_null() {
        return REG_NOMATCH;
    }
    let prog: &Prog = &*priv_.cast::<Prog>();
    let s: &[u8] = core::ffi::CStr::from_ptr(string).to_bytes();
    let notbol: bool = eflags & REG_NOTBOL != 0;
    let noteol: bool = eflags & REG_NOTEOL != 0;

    match exec(prog, s, notbol, noteol) {
        None => REG_NOMATCH,
        Some(matched) => {
            let nosub: bool = (*preg).cflags & REG_NOSUB != 0;
            if !nosub && nmatch > 0 && !pmatch.is_null() {
                for i in 0..nmatch {
                    let so: i32 = matched.get(2 * i).copied().unwrap_or(-1);
                    let eo: i32 = matched.get(2 * i + 1).copied().unwrap_or(-1);
                    let m: &mut regmatch_t = &mut *pmatch.add(i);
                    // Widen the engine's internal `i32` offsets to the public `regoff_t`.
                    m.rm_so = regoff_t::try_from(so).unwrap_or(-1);
                    m.rm_eo = regoff_t::try_from(eo).unwrap_or(-1);
                }
            }
            REG_NOERROR
        },
    }
}
