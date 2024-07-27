// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() {
    nvx::log!("Running init server...");
    let magic_string: &str = "PANIC: Hello World!\n";
    let _ = ::nvx::debug::debug(magic_string.as_ptr(), magic_string.len());
    loop {
        core::hint::spin_loop()
    }
}
