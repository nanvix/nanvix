// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    fmt::Write,
    hint,
    panic::PanicMessage,
};
use syslog::{
    LogLevel,
    Logger,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[panic_handler]
pub fn panic_implementation(info: &::core::panic::PanicInfo<'_>) -> ! {
    // Extract panic information.
    let (file, line) = match info.location() {
        Some(loc) => (loc.file(), loc.line()),
        None => ("", 0),
    };

    // Print panic information.
    let m: PanicMessage<'_> = info.message();
    let _ = writeln!(
        &mut Logger::get(module_path!(), LogLevel::Trace),
        "PANIC file='{file}', line={line} :: {m}",
    );

    loop {
        hint::spin_loop()
    }
}
