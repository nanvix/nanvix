// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::klog::{
    Klog,
    KlogLevel,
};
use ::core::{
    fmt::Write,
    hint,
    panic::PanicInfo,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Panics the kernel, potentially dumping information about what happened.
///
/// # Parameters
///
/// - `info`: Information about the panic.
///
#[panic_handler]
pub fn kpanic(info: &PanicInfo) -> ! {
    // Create anonymous scope to release reference to kernel log.
    {
        let klog: &mut Klog = &mut crate::klog::Klog::get("kernel", KlogLevel::Panic);
        // Extract panic information.
        let (file, line): (&str, u32) = match info.location() {
            Some(location) => (location.file(), location.line()),
            None => ("", 0),
        };

        // Print panic information.
        if let Some(m) = info.message() {
            let _ = write!(klog, "file='{}', line={} :: {}", file, line, m);
        } else if let Some(m) = info.payload().downcast_ref::<&str>() {
            let _ = write!(klog, "file='{}', line={} :: {}", file, line, m);
        } else {
            let _ = write!(klog, "file='{}', line={} :: ?", file, line);
        }
    }

    loop {
        hint::spin_loop();
    }
}
