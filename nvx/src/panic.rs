// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::log;
use ::core::hint;

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
    if let Some(m) = info.message() {
        log!("PANIC file='{}', line={} :: {}", file, line, m);
    } else if let Some(m) = info.payload().downcast_ref::<&str>() {
        log!("PANIC file='{}', line={} :: {}", file, line, m);
    } else {
        log!("PANIC file='{}', line={} :: ?", file, line);
    }
    loop {
        hint::spin_loop()
    }
}
