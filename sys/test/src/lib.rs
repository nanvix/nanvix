// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() {
    libnanvix::log!("Running test server...");
    loop {
        core::hint::spin_loop()
    }
}
