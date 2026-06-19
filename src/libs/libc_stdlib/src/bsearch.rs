// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ptr::null_mut;
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Searches an array of `nmemb` elements for a member that matches `key`, using binary search.
/// The array must be sorted in ascending order according to the comparison function.
///
/// # Parameters
///
/// - `key`: Pointer to the key to search for.
/// - `base`: Pointer to the first element of the sorted array.
/// - `nmemb`: Number of elements in the array.
/// - `size`: Size of each element in bytes.
/// - `compar`: Comparison function.
///
/// # Returns
///
/// A pointer to a matching element, or null if no match is found.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers and calls a function pointer.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/bsearch.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn bsearch(
    key: *const c_void,
    base: *const c_void,
    nmemb: c_size_t,
    size: c_size_t,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
) -> *mut c_void {
    let compar = match compar {
        Some(f) => f,
        None => return null_mut(),
    };

    if key.is_null() || base.is_null() || nmemb == 0 || size == 0 {
        return null_mut();
    }

    let base_ptr: *const u8 = base.cast::<u8>();
    let size = size as usize;
    let mut low: usize = 0;
    let mut high: usize = nmemb as usize;

    while low < high {
        let mid = low + (high - low) / 2;
        let elem = base_ptr.add(mid * size).cast::<c_void>();
        let cmp = compar(key, elem);

        if cmp < 0 {
            high = mid;
        } else if cmp > 0 {
            low = mid + 1;
        } else {
            return elem.cast_mut();
        }
    }

    null_mut()
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::bsearch;
    use ::sysapi::ffi::{
        c_int,
        c_void,
    };

    unsafe extern "C" fn compare_ints(a: *const c_void, b: *const c_void) -> c_int {
        let a_val: c_int = *a.cast::<c_int>();
        let b_val: c_int = *b.cast::<c_int>();
        if a_val < b_val {
            -1
        } else if a_val > b_val {
            1
        } else {
            0
        }
    }

    #[test]
    fn found() {
        let arr: [c_int; 5] = [1, 2, 3, 4, 5];
        let key: c_int = 3;
        let result = unsafe {
            bsearch(
                (&key as *const c_int).cast::<c_void>(),
                arr.as_ptr().cast::<c_void>(),
                5,
                4,
                Some(compare_ints),
            )
        };
        assert!(!result.is_null());
        assert_eq!(unsafe { *result.cast::<c_int>() }, 3);
    }

    #[test]
    fn not_found() {
        let arr: [c_int; 5] = [1, 2, 3, 4, 5];
        let key: c_int = 6;
        let result = unsafe {
            bsearch(
                (&key as *const c_int).cast::<c_void>(),
                arr.as_ptr().cast::<c_void>(),
                5,
                4,
                Some(compare_ints),
            )
        };
        assert!(result.is_null());
    }

    #[test]
    fn empty_array() {
        let arr: [c_int; 0] = [];
        let key: c_int = 1;
        let result = unsafe {
            bsearch(
                (&key as *const c_int).cast::<c_void>(),
                arr.as_ptr().cast::<c_void>(),
                0,
                4,
                Some(compare_ints),
            )
        };
        assert!(result.is_null());
    }
}
