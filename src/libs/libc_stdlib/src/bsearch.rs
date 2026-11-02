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
mod tests;
