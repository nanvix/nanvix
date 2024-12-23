// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::c_int;

cfg_if::cfg_if! {
    if #[cfg(all(feature = "syscall", feature = "staticlib"))] {

        extern "C" {
            pub static mut errno: c_int;
        }
    } else {
        #[allow(non_upper_case_globals)]
        pub static mut errno: c_int = 0;
    }
}
