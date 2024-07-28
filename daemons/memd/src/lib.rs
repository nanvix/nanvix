// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() {
    ::nvx::log!("running memory management daemon...");

    loop {
        ::core::hint::spin_loop();
    }
}
