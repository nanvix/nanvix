// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#[unsafe(no_mangle)]
extern "C" fn __errno_location() -> *mut ::sysapi::ffi::c_int {
    thread_local! {
        static ERRNO: ::core::cell::UnsafeCell<::sysapi::ffi::c_int> =
            const { ::core::cell::UnsafeCell::new(0) };
    }
    ERRNO.with(|cell| cell.get())
}
