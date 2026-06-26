// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_long;
use ::syslog::trace_syscall;

//==================================================================================================
// Constants
//==================================================================================================

/// Host identifier reported by Nanvix. Encodes the loopback-derived address `127.1.1`.
const NANVIX_HOST_ID: c_long = 0x007f_0101;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the unique 32-bit identifier of the current host. Nanvix does not persist a configurable
/// host ID, so a fixed value derived from the loopback address is reported. This is sufficient for
/// applications that only require a stable, non-zero identifier.
///
/// # Returns
///
/// The host identifier of the current machine.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn gethostid() -> c_long {
    NANVIX_HOST_ID
}
