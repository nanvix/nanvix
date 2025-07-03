// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Global Variables
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(all(feature = "syscall", feature = "staticlib"))] {
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
        #[allow(non_upper_case_globals)]
        static mut errno: c_int = 0;

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
        pub unsafe fn __errno_location() -> *mut c_int {
            &raw mut errno as *mut c_int
        }
    }
}

#[cfg(all(feature = "syscall"))]
mod bindings {
    use ::sysapi::ffi::c_int;

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn __errno_location() -> *mut c_int {
        crate::errno::__errno_location()
    }
}