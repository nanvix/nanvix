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
/// Nanvix computes the result at `double` precision and the generated C declaration returns
/// `double` to match. This keeps the implementation and the C prototype in agreement on the
/// return-value ABI (a `long double` prototype would expect the value in the x87 register on the
/// supported targets, corrupting callers).
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
