// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem::align_of;
use ::sysapi::{
    ffi::c_char,
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Size-bounded string concatenation.
///
/// This function appends the string `src` to the end of `dest`. It will append at most
/// `size - strlen(dest) - 1` bytes, null-terminating the result.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination null-terminated string.
/// - `src`: Pointer to the source null-terminated string.
/// - `size`: Full size of the destination buffer.
///
/// # Return Value
///
/// Returns `strlen(dest) + strlen(src)` (initial lengths), representing the total length of the
/// string it tried to create. If the return value is >= `size`, truncation occurred.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It writes to the memory region pointed to by `dest` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strlcat(
    dest: *mut c_char,
    src: *const c_char,
    size: c_size_t,
) -> c_size_t {
    debug_assert!(!dest.is_null(), "strlcat(): null destination pointer");
    debug_assert!(!src.is_null(), "strlcat(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_char>()),
        "strlcat(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_char>()),
        "strlcat(): source pointer is not properly aligned"
    );

    let size: usize = size as usize;

    // Find current length of dest (up to size), using c_size_t to avoid truncation casts.
    let mut dest_len: c_size_t = 0;
    while (dest_len as usize) < size && *dest.add(dest_len as usize) != 0 {
        dest_len += 1;
    }

    // Calculate strlen(src).
    let mut src_len: c_size_t = 0;
    while *src.add(src_len as usize) != 0 {
        src_len += 1;
    }

    // If dest_len >= size, dest is not null-terminated within size.
    if (dest_len as usize) >= size {
        return dest_len + src_len;
    }

    // Append at most size - dest_len - 1 bytes from src.
    let remaining: usize = size - (dest_len as usize) - 1;
    let copy_len: usize = if (src_len as usize) < remaining {
        src_len as usize
    } else {
        remaining
    };
    let mut i: usize = 0;
    while i < copy_len {
        *dest.add((dest_len as usize) + i) = *src.add(i);
        i += 1;
    }
    *dest.add((dest_len as usize) + copy_len) = 0 as c_char;

    dest_len + src_len
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strlcat;
    use ::std::vec::Vec;
    use ::sysapi::{
        ffi::c_char,
        sys_types::c_size_t,
    };

    fn make_c_string(bytes: &[u8]) -> Vec<c_char> {
        let mut v: Vec<c_char> = bytes
            .iter()
            .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
            .collect();
        v.push(0 as c_char);
        v
    }

    #[test]
    fn test_strlcat_normal() {
        let mut dest: Vec<c_char> = vec![0 as c_char; 20];
        let hello: Vec<c_char> = make_c_string(b"hello");
        for (i, &c) in hello.iter().enumerate() {
            dest[i] = c;
        }
        let src: Vec<c_char> = make_c_string(b" world");
        let ret: c_size_t = unsafe { strlcat(dest.as_mut_ptr(), src.as_ptr(), 20) };
        assert_eq!(ret as usize, 11, "strlcat returns strlen(dest) + strlen(src)");
        assert_eq!(dest[0], c_char::try_from(b'h').expect("ASCII fits in c_char"));
        assert_eq!(dest[5], c_char::try_from(b' ').expect("ASCII fits in c_char"));
        assert_eq!(dest[11], 0 as c_char);
    }

    #[test]
    fn test_strlcat_truncation() {
        let mut dest: Vec<c_char> = vec![0 as c_char; 8];
        let hello: Vec<c_char> = make_c_string(b"hello");
        for (i, &c) in hello.iter().enumerate() {
            dest[i] = c;
        }
        let src: Vec<c_char> = make_c_string(b" world");
        let ret: c_size_t = unsafe { strlcat(dest.as_mut_ptr(), src.as_ptr(), 8) };
        assert_eq!(ret as usize, 11, "strlcat returns intended total length");
        assert_eq!(dest[7], 0 as c_char, "dest should be null-terminated");
    }
}
