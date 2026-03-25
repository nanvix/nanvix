// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ffi::c_void;
use ::sys::error::Error;
use ::sysapi::sys_types::c_size_t;

//==================================================================================================
// External C Functions
//==================================================================================================

unsafe extern "C" {
    fn malloc(size: c_size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn aligned_alloc(alignment: c_size_t, size: c_size_t) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: c_size_t) -> *mut c_void;
    fn malloc_usable_size(ptr: *mut c_void) -> c_size_t;
    fn memset(s: *mut c_void, c: i32, n: c_size_t) -> *mut c_void;
}

//==================================================================================================
// Constants
//==================================================================================================

/// Pattern byte used to fill memory.
const FILL_BYTE: u8 = 0xA5;

/// Number of grow/shrink cycles in the realloc stress test.
const STRESS_CYCLES: usize = 64;

//==================================================================================================
// Helpers
//==================================================================================================

/// Returns the raw address of a pointer as `usize`.
#[allow(clippy::as_conversions)]
fn ptr_addr(p: *mut c_void) -> usize {
    p as usize
}

/// Converts a `c_size_t` to `usize` (infallible on this 32-bit platform).
#[allow(clippy::as_conversions)]
fn sz(v: c_size_t) -> usize {
    v as usize
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Executes all allocator tests.
pub fn run() -> Result<(), Error> {
    test_malloc_free()?;
    test_aligned_alloc_free()?;
    test_realloc()?;
    test_malloc_usable_size()?;
    Ok(())
}

/// Tests whether we can allocate and free memory using `malloc()` and `free()`.
fn test_malloc_free() -> Result<(), Error> {
    let sizes: &[c_size_t] = &[1, 8, 32, 128, 511, 512, 1024, 4096];

    // Allocate, touch, and free each block individually.
    for &s in sizes {
        let ptr: *mut c_void = unsafe { malloc(s) };
        assert!(!ptr.is_null(), "malloc({s}) returned null");

        // Fill memory with a pattern and verify it was written.
        unsafe { memset(ptr, 0xA5, s) };
        let bytes: *const u8 = ptr.cast::<u8>();
        for j in 0..sz(s) {
            assert_eq!(unsafe { *bytes.add(j) }, 0xA5);
        }

        unsafe { free(ptr) };
    }

    // Repeated allocate/free cycle to check for simple leaks or reuse issues.
    for iter in 0u8..100 {
        let p: *mut c_void = unsafe { malloc(64) };
        assert!(!p.is_null(), "malloc(64) returned null on iteration {iter}");
        unsafe { *p.cast::<u8>() = iter };
        unsafe { free(p) };
    }

    Ok(())
}

/// Tests whether we can allocate and free aligned memory using `aligned_alloc()` and `free()`.
fn test_aligned_alloc_free() -> Result<(), Error> {
    // (alignment, size) pairs. Size must be a multiple of alignment.
    let cases: &[(c_size_t, c_size_t)] = &[
        (4, 4),
        (8, 16),
        (16, 32),
        (32, 64),
        (64, 128),
        (128, 256),
        (256, 512),
        (512, 512),
        (1024, 2048),
    ];

    for &(alignment, size) in cases {
        let align_usize: usize = sz(alignment);
        let size_usize: usize = sz(size);

        // Sanity checks matching the C test.
        assert_eq!(
            align_usize & (align_usize.wrapping_sub(1)),
            0,
            "alignment must be power of two"
        );
        assert_eq!(size_usize % align_usize, 0, "size must be multiple of alignment");

        let ptr: *mut c_void = unsafe { aligned_alloc(alignment, size) };
        assert!(!ptr.is_null(), "aligned_alloc({alignment}, {size}) returned null");

        let addr: usize = ptr_addr(ptr);
        assert_eq!(addr % align_usize, 0, "pointer not aligned to {alignment}");

        // Fill memory with a pattern and verify it was written.
        unsafe { memset(ptr, 0x5A, size) };
        let bytes: *const u8 = ptr.cast::<u8>();
        for j in 0..size_usize {
            assert_eq!(unsafe { *bytes.add(j) }, 0x5A);
        }

        unsafe { free(ptr) };
    }

    // Stress: allocate and free many aligned blocks of the same alignment.
    for iter in 0u8..64 {
        let p: *mut c_void = unsafe { aligned_alloc(64, 64) };
        assert!(!p.is_null(), "aligned_alloc(64, 64) returned null on iteration {iter}");
        let addr: usize = ptr_addr(p);
        assert_eq!(addr % 64, 0, "pointer not aligned to 64");
        unsafe { *p.cast::<u8>() = iter };
        unsafe { free(p) };
    }

    Ok(())
}

/// Tests realloc growth, shrink, NULL-ptr, data preservation, and alignment boundary sizes.
fn test_realloc() -> Result<(), Error> {
    // Growth path: malloc then realloc to a larger size.
    {
        let ptr: *mut c_void = unsafe { malloc(64) };
        assert!(!ptr.is_null());
        unsafe { memset(ptr, i32::from(FILL_BYTE), 64) };

        let ptr2: *mut c_void = unsafe { realloc(ptr, 256) };
        assert!(!ptr2.is_null());

        // Verify original data is preserved in the first 64 bytes.
        let bytes: *const u8 = ptr2.cast::<u8>();
        for i in 0..64usize {
            assert_eq!(unsafe { *bytes.add(i) }, FILL_BYTE);
        }

        unsafe { free(ptr2) };
    }

    // Shrink path: malloc a large block then realloc to a smaller size.
    {
        let ptr: *mut c_void = unsafe { malloc(512) };
        assert!(!ptr.is_null());
        unsafe { memset(ptr, i32::from(FILL_BYTE), 512) };

        let ptr2: *mut c_void = unsafe { realloc(ptr, 64) };
        assert!(!ptr2.is_null());

        // Verify data preserved in the smaller region.
        let bytes: *const u8 = ptr2.cast::<u8>();
        for i in 0..64usize {
            assert_eq!(unsafe { *bytes.add(i) }, FILL_BYTE);
        }

        unsafe { free(ptr2) };
    }

    // NULL-ptr realloc: behaves like malloc.
    {
        let ptr: *mut c_void = unsafe { realloc(core::ptr::null_mut(), 128) };
        assert!(!ptr.is_null());
        unsafe { memset(ptr, i32::from(FILL_BYTE), 128) };

        let bytes: *const u8 = ptr.cast::<u8>();
        for i in 0..128usize {
            assert_eq!(unsafe { *bytes.add(i) }, FILL_BYTE);
        }

        unsafe { free(ptr) };
    }

    // Alignment boundary sizes: exercise UNDERLYING_ALIGNMENT (8-byte) rounding.
    {
        let sizes: &[c_size_t] = &[1, 7, 8, 9, 15, 16, 17, 31, 32, 33];
        for &s in sizes {
            let s_usize: usize = sz(s);
            let ptr: *mut c_void = unsafe { malloc(s) };
            assert!(!ptr.is_null());
            unsafe { memset(ptr, i32::from(FILL_BYTE), s) };

            // Grow to double size.
            let new_sz: c_size_t = s.wrapping_mul(2);
            let ptr2: *mut c_void = unsafe { realloc(ptr, new_sz) };
            assert!(!ptr2.is_null());

            // Verify original data preserved.
            let bytes: *const u8 = ptr2.cast::<u8>();
            for j in 0..s_usize {
                assert_eq!(unsafe { *bytes.add(j) }, FILL_BYTE);
            }

            unsafe { free(ptr2) };
        }
    }

    // Repeated grow/shrink cycles: stress alignment and header validation.
    {
        let mut cur_size: c_size_t = 32;
        let ptr: *mut c_void = unsafe { malloc(cur_size) };
        assert!(!ptr.is_null());
        unsafe { memset(ptr, i32::from(FILL_BYTE), cur_size) };

        let mut current: *mut c_void = ptr;

        for cycle in 0..STRESS_CYCLES {
            // Alternate: grow on even cycles, shrink on odd.
            let mut new_size: c_size_t = if cycle % 2 == 0 {
                cur_size.wrapping_mul(2)
            } else {
                cur_size / 2
            };
            if new_size == 0 {
                new_size = 1;
            }

            let check_size: usize = if new_size < cur_size {
                sz(new_size)
            } else {
                sz(cur_size)
            };

            let ptr2: *mut c_void = unsafe { realloc(current, new_size) };
            assert!(!ptr2.is_null());

            // Verify data preserved up to the smaller of old/new sizes.
            let bytes: *const u8 = ptr2.cast::<u8>();
            for j in 0..check_size {
                assert_eq!(unsafe { *bytes.add(j) }, FILL_BYTE);
            }

            // Fill the new region with the pattern.
            unsafe { memset(ptr2, i32::from(FILL_BYTE), new_size) };

            current = ptr2;
            cur_size = new_size;
        }

        unsafe { free(current) };
    }

    Ok(())
}

/// Tests whether we can query the usable size of allocated blocks using `malloc_usable_size()`.
fn test_malloc_usable_size() -> Result<(), Error> {
    // Null pointer must yield zero.
    {
        let s: c_size_t = unsafe { malloc_usable_size(core::ptr::null_mut()) };
        assert_eq!(s, 0);
    }

    // Exercise a range of allocation sizes and verify reported usable size.
    {
        let sizes: &[c_size_t] = &[1, 8, 32, 64, 127, 128, 255, 256, 511, 512, 1024];
        for &req in sizes {
            let req_usize: usize = sz(req);
            let p: *mut c_void = unsafe { malloc(req) };
            assert!(!p.is_null());

            let usable: c_size_t = unsafe { malloc_usable_size(p) };
            assert!(usable >= req, "usable size {usable} < requested {req}");
            assert_eq!(usable, req, "usable size {usable} != requested {req}");

            // Touch within requested bounds.
            unsafe { memset(p, 0xA5, req) };
            let bytes: *const u8 = p.cast::<u8>();
            for j in 0..req_usize {
                assert_eq!(unsafe { *bytes.add(j) }, 0xA5);
            }

            unsafe { free(p) };
        }
    }

    // Realloc growth path: ensure updated usable size reflects new request.
    {
        let p: *mut c_void = unsafe { malloc(64) };
        assert!(!p.is_null());
        assert_eq!(unsafe { malloc_usable_size(p) }, 64);

        let p2: *mut c_void = unsafe { realloc(p, 200) };
        assert!(!p2.is_null());
        assert_eq!(unsafe { malloc_usable_size(p2) }, 200);

        unsafe { memset(p2, 0x5A, 200) };
        unsafe { free(p2) };
    }

    // Aligned allocation path.
    {
        let pa: *mut c_void = unsafe { aligned_alloc(128, 256) };
        assert!(!pa.is_null());
        let addr: usize = ptr_addr(pa);
        assert_eq!(addr % 128, 0, "pointer not aligned to 128");
        assert_eq!(unsafe { malloc_usable_size(pa) }, 256);
        unsafe { free(pa) };
    }

    // Stress: allocate/free and query usable size repeatedly to reveal metadata issues.
    for iter in 0u8..100 {
        let p: *mut c_void = unsafe { malloc(96) };
        assert!(!p.is_null());
        assert_eq!(unsafe { malloc_usable_size(p) }, 96);
        unsafe { *p.cast::<u8>() = iter };
        unsafe { free(p) };
    }

    Ok(())
}
