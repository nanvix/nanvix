// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::core::cell::Cell;
use ::sysapi::ffi::c_int;

std::thread_local! {
    static ERRNO: Cell<c_int> = const { Cell::new(0) };
}

/// Host-only definition of `__errno_location` for Windows `std` unit tests.
///
/// # Safety
///
/// The returned pointer is valid for the lifetime of the calling thread.
#[unsafe(no_mangle)]
unsafe extern "C" fn __errno_location() -> *mut c_int {
    ERRNO.with(Cell::as_ptr)
}
