// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    arch::asm,
    fmt::Write,
    panic::PanicMessage,
};
use syslog::{
    LogLevel,
    Logger,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

// When built with the opt-in `weak-panic` feature, the panic handler is given
// **weak** linkage so the `rust_begin_unwind` entry emitted into each separate
// Nanvix static archive (`libc.a`, `libm.a`) collapses to a single definition
// at the final guest link instead of colliding as multiple *strong* symbols
// (which would otherwise force `-z muldefs` / `--allow-multiple-definition`).
//
// The feature is OFF by default: a regular guest binary links `nvx` exactly
// once and therefore keeps a single strong panic handler, unchanged from the
// historical behaviour.  Only the in-tree `nanvix_libc` / `nanvix_libm`
// staticlib bundle (which links `nvx` into more than one archive) enables it.
#[panic_handler]
#[cfg_attr(feature = "weak-panic", linkage = "weak")]
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

    // Trigger an invalid-opcode exception so the kernel terminates this process instead of letting
    // it spin forever.
    unsafe {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        asm!("ud2", options(noreturn));

        #[cfg(target_arch = "aarch64")]
        asm!("udf #0", options(noreturn));
    }
}
