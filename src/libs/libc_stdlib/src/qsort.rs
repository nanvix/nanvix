// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Private Functions
//==================================================================================================

/// Swaps two elements of `size` bytes at the given pointers.
///
/// # Safety
///
/// Both `a` and `b` must point to valid, non-overlapping memory regions of at least `size` bytes.
unsafe fn swap_elements(a: *mut u8, b: *mut u8, size: usize) {
    for i in 0..size {
        let tmp = *a.add(i);
        *a.add(i) = *b.add(i);
        *b.add(i) = tmp;
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sorts an array of `nmemb` elements, each of `size` bytes, using the comparison function
/// pointed to by `compar`. This implementation uses insertion sort for simplicity.
///
/// # Parameters
///
/// - `base`: Pointer to the first element of the array.
/// - `nmemb`: Number of elements in the array.
/// - `size`: Size of each element in bytes.
/// - `compar`: Comparison function returning negative, zero, or positive.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers and calls a function pointer.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/qsort.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn qsort(
    base: *mut c_void,
    nmemb: c_size_t,
    size: c_size_t,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
) {
    let compar = match compar {
        Some(f) => f,
        None => return,
    };

    if base.is_null() || nmemb <= 1 || size == 0 {
        return;
    }

    let base_ptr: *mut u8 = base.cast::<u8>();
    let size = size as usize;
    let nmemb = nmemb as usize;

    // Insertion sort.
    for i in 1..nmemb {
        let mut j = i;
        while j > 0 {
            let a = base_ptr.add(j * size);
            let b = base_ptr.add((j - 1) * size);
            if compar(a.cast_const().cast::<c_void>(), b.cast_const().cast::<c_void>()) < 0 {
                swap_elements(a, b, size);
                j -= 1;
            } else {
                break;
            }
        }
    }
}

///
/// # Description
///
/// Sorts an array like [`qsort`], passing `arg` through to the comparison function.
///
/// # Parameters
///
/// - `base`: Pointer to the first element of the array.
/// - `nmemb`: Number of elements in the array.
/// - `size`: Size of each element in bytes.
/// - `compar`: Comparison function returning negative, zero, or positive.
/// - `arg`: Opaque context pointer passed to `compar`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers and calls a function pointer.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/qsort.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn qsort_r(
    base: *mut c_void,
    nmemb: c_size_t,
    size: c_size_t,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void) -> c_int>,
    arg: *mut c_void,
) {
    let compar = match compar {
        Some(f) => f,
        None => return,
    };

    if base.is_null() || nmemb <= 1 || size == 0 {
        return;
    }

    let base_ptr: *mut u8 = base.cast::<u8>();
    let size = size as usize;
    let nmemb = nmemb as usize;

    for i in 1..nmemb {
        let mut j = i;
        while j > 0 {
            let a = base_ptr.add(j * size);
            let b = base_ptr.add((j - 1) * size);
            if compar(a.cast_const().cast::<c_void>(), b.cast_const().cast::<c_void>(), arg) < 0 {
                swap_elements(a, b, size);
                j -= 1;
            } else {
                break;
            }
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{
        qsort,
        qsort_r,
    };
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
    fn sort_integers() {
        let mut arr: [c_int; 5] = [5, 3, 1, 4, 2];
        unsafe {
            qsort(arr.as_mut_ptr().cast::<c_void>(), 5, 4, Some(compare_ints));
        }
        assert_eq!(arr, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn empty_array() {
        let mut arr: [c_int; 0] = [];
        unsafe {
            qsort(arr.as_mut_ptr().cast::<c_void>(), 0, 4, Some(compare_ints));
        }
        assert_eq!(arr, []);
    }

    #[test]
    fn single_element() {
        let mut arr: [c_int; 1] = [42];
        unsafe {
            qsort(arr.as_mut_ptr().cast::<c_void>(), 1, 4, Some(compare_ints));
        }
        assert_eq!(arr, [42]);
    }

    #[test]
    fn already_sorted() {
        let mut arr: [c_int; 4] = [1, 2, 3, 4];
        unsafe {
            qsort(arr.as_mut_ptr().cast::<c_void>(), 4, 4, Some(compare_ints));
        }
        assert_eq!(arr, [1, 2, 3, 4]);
    }

    unsafe extern "C" fn compare_ints_with_direction(
        a: *const c_void,
        b: *const c_void,
        arg: *mut c_void,
    ) -> c_int {
        let direction: c_int = *arg.cast::<c_int>();
        compare_ints(a, b) * direction
    }

    #[test]
    fn sort_with_context() {
        let mut arr: [c_int; 5] = [5, 3, 1, 4, 2];
        let mut direction: c_int = -1;
        unsafe {
            qsort_r(
                arr.as_mut_ptr().cast::<c_void>(),
                5,
                4,
                Some(compare_ints_with_direction),
                (&mut direction as *mut c_int).cast::<c_void>(),
            );
        }
        assert_eq!(arr, [5, 4, 3, 2, 1]);
    }
}
