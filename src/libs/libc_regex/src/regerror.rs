// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::types::{
    regex_t,
    REG_BADBR,
    REG_BADPAT,
    REG_BADRPT,
    REG_EBRACE,
    REG_EBRACK,
    REG_ECOLLATE,
    REG_ECTYPE,
    REG_EESCAPE,
    REG_EPAREN,
    REG_ERANGE,
    REG_ESPACE,
    REG_ESUBREG,
    REG_NOMATCH,
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
/// Produces a human-readable message for the regex error code `errcode`.
///
/// # Parameters
///
/// - `errcode`: A `REG_*` error code returned by `regcomp()` or `regexec()`.
/// - `_preg`: Unused; accepted for POSIX compatibility.
/// - `errbuf`: Optional buffer that receives the NUL-terminated message.
/// - `errbuf_size`: Size of `errbuf`, in bytes.
///
/// # Returns
///
/// Returns the size of the buffer needed to hold the message, including the terminating NUL.
///
/// # Safety
///
/// This function is unsafe because it writes through the raw pointer `errbuf`. The caller must
/// ensure that `errbuf` (if non-null) has room for `errbuf_size` bytes.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn regerror(
    errcode: c_int,
    _preg: *const regex_t,
    errbuf: *mut c_char,
    errbuf_size: usize,
) -> usize {
    let msg: &[u8] = match errcode {
        REG_NOMATCH => b"no match",
        REG_BADPAT => b"invalid regular expression",
        REG_ECOLLATE => b"invalid collating element",
        REG_EBRACK => b"unbalanced [",
        REG_EPAREN => b"unbalanced (",
        REG_EBRACE => b"unbalanced {",
        REG_BADBR => b"invalid {} content",
        REG_ERANGE => b"invalid range",
        REG_ECTYPE => b"invalid character class",
        REG_EESCAPE => b"trailing backslash",
        REG_ESUBREG => b"invalid back reference",
        REG_ESPACE => b"out of memory",
        REG_BADRPT => b"invalid repetition",
        _ => b"regex error",
    };
    let len: usize = msg.len();
    if !errbuf.is_null() && errbuf_size > 0 {
        let n: usize = if len < errbuf_size - 1 {
            len
        } else {
            errbuf_size - 1
        };
        for (i, &b) in msg.iter().take(n).enumerate() {
            *errbuf.add(i).cast::<u8>() = b;
        }
        *errbuf.add(n).cast::<u8>() = 0;
    }
    len + 1
}
