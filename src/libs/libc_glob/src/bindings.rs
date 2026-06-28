// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_char,
    c_int,
    c_void,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Constants
//==================================================================================================

/// Subset of `GLOB_*` flag bits from `<glob.h>` consulted by the [`glob()`] stub.
const GLOB_DOOFFS: c_int = 0x0008;
const GLOB_APPEND: c_int = 0x0020;
/// Return value from `<glob.h>` indicating that a pattern matched no paths.
const GLOB_NOMATCH: c_int = 3;

//==================================================================================================
// Structures
//==================================================================================================

/// Mirror of the C `glob_t` result structure declared in `<glob.h>`.
///
/// The fields are written through a raw pointer to hand callers a well-defined empty result; they
/// match the C ABI layout `{ size_t gl_pathc; char **gl_pathv; size_t gl_offs; }`.
#[repr(C)]
struct GlobT {
    gl_pathc: usize,
    gl_pathv: *mut *mut c_char,
    gl_offs: usize,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Searches for paths matching a shell wildcard pattern. Nanvix does not provide pathname globbing,
/// so this stub reports that no paths matched.
///
/// # Parameters
///
/// - `pattern`: Null-terminated wildcard pattern (ignored).
/// - `flags`: Bitwise-or of `GLOB_*` flags. `GLOB_APPEND` and `GLOB_DOOFFS` are honored when
///   clearing `pglob`; all other flags are ignored.
/// - `errfunc`: Optional error callback (ignored).
/// - `pglob`: Pointer to a `glob_t` result structure. Cleared to an empty result when non-null
///   and `GLOB_APPEND` is not set.
///
/// # Returns
///
/// `GLOB_NOMATCH` (3), indicating that the pattern matched no paths. Callers such as the hush shell
/// then treat the pattern as a literal string.
///
/// # Notes
///
/// This is a dummy implementation that always reports no match; behavioral flags such as
/// `GLOB_NOCHECK` (which would otherwise return the pattern itself) are not honored. A future
/// version should expand the pattern using `fnmatch()` over directory entries and populate
/// `pglob->gl_pathv` / `pglob->gl_pathc`.
///
/// # Safety
///
/// `pglob` must be either null or a valid, writable pointer to a `glob_t`. When `GLOB_DOOFFS`
/// is set (and `GLOB_APPEND` is not), the caller must also have initialized `gl_offs`, which is
/// read before the structure is rewritten. `pattern` and `errfunc` are ignored.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn glob(
    _pattern: *const c_char,
    flags: c_int,
    _errfunc: Option<extern "C" fn(epath: *const c_char, eerrno: c_int) -> c_int>,
    pglob: *mut c_void,
) -> c_int {
    ::syslog::debug!("glob(): not implemented");

    // glibc and the BSDs leave `gl_pathc`/`gl_pathv` in a well-defined empty state even on a
    // no-match, so portable callers may inspect them without pre-zeroing the structure. Mirror
    // that contract to avoid exposing uninitialized memory. An existing result list is preserved
    // when GLOB_APPEND is set, and a caller-provided `gl_offs` is preserved under GLOB_DOOFFS.
    if !pglob.is_null() && (flags & GLOB_APPEND) == 0 {
        let pglob: *mut GlobT = pglob.cast();
        let gl_offs: usize = if (flags & GLOB_DOOFFS) != 0 {
            (*pglob).gl_offs
        } else {
            0
        };
        core::ptr::write(
            pglob,
            GlobT {
                gl_pathc: 0,
                gl_pathv: core::ptr::null_mut(),
                gl_offs,
            },
        );
    }

    // Report that the pattern matched nothing so callers such as the hush shell fall back to
    // treating the pattern as a literal string. Returning GLOB_NOSYS would instead be fatal for
    // such callers, so GLOB_NOMATCH is intentional.
    GLOB_NOMATCH
}

///
/// # Description
///
/// Frees the dynamic storage allocated by a successful [`glob()`] call. Because [`glob()`] never
/// allocates (it always reports no match), this is a no-op.
///
/// # Parameters
///
/// - `pglob`: Pointer to the `glob_t` structure to release (ignored).
///
/// # Safety
///
/// This function is safe to call with any argument; it does nothing.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn globfree(_pglob: *mut c_void) {
    ::syslog::debug!("globfree(): not implemented");
}
