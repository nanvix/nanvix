// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::log;
use ::core::{
    hint,
    panic::PanicMessage,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[panic_handler]
pub fn panic_implementation(info: &::core::panic::PanicInfo) -> ! {
    // Extract panic information.
    let (file, line) = match info.location() {
        Some(loc) => (loc.file(), loc.line()),
        None => ("", 0),
    };

    // Print panic information.
    let m: PanicMessage = info.message();
    log!("PANIC file='{}', line={} :: {}", file, line, m);

    loop {
        hint::spin_loop()
    }
}
