// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(all(feature = "syscall", feature = "staticlib"))] {
        use ::sysapi::ffi::c_int;

        unsafe extern "C" {
            pub fn __errno() -> *mut c_int;
        }

        ///
        /// # Description
        ///
        /// Returns a pointer to `errno` variable.
        ///
        /// # Returns
        ///
        /// A mutable pointer to the `errno` variable.
        ///
        /// # Safety
        ///
        /// This function is unsafe because it may interoperate with external code.
        ///
        pub unsafe fn  __errno_location() -> *mut c_int {
            __errno()
        }
    } else {
        pub use ::syscall::errno::__errno_location;
    }
}
