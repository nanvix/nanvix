// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::platform,
    klog::{
        Klog,
        KlogLevel,
    },
};
use ::core::{
    fmt::Write,
    panic::{
        PanicInfo,
        PanicMessage,
    },
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
        let m: PanicMessage = info.message();
        let _ = write!(klog, "file='{}', line={} :: {}", file, line, m);
    }

    platform::shutdown();
}
