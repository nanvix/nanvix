// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_char;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts the initial portion of the string pointed to by `nptr` to a floating-point value.
///
/// The C prototype returns `long double` to match POSIX. The conversion is computed at `double`
/// precision and delegated to [`crate::strtod`]. This is ABI-correct on the supported i686 target,
/// where the cdecl convention returns `float`, `double`, and `long double` alike in the x87 `st0`
/// register; the `f64` result is promoted to the 80-bit extended representation on return, so the
/// value the caller pops as `long double` is exactly the converted number.
///
/// # Parameters
///
/// - `nptr`: Pointer to the null-terminated string to be converted.
/// - `endptr`: If not null, receives a pointer to the first character not converted.
///
/// # Returns
///
/// The converted floating-point value.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers through [`crate::strtod`].
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/strtod.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtold(nptr: *const c_char, endptr: *mut *mut c_char) -> f64 {
    crate::strtod(nptr, endptr)
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::strtold;
    use ::sysapi::ffi::c_char;

    #[test]
    fn delegates_to_strtod() {
        let s = b"0x1.8p+1\0";
        let value: f64 = unsafe { strtold(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert_eq!(value, 3.0);
    }
}
