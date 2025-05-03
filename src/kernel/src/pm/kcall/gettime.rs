// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::{
        KcallArgs,
        KcallResult,
    },
    pm::{
        clock,
        ProcessManager,
    },
};
use ::sys::{
    error::Error,
    mm::VirtualAddress,
    pm::ProcessIdentifier,
};
use ::time::SystemTime;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Gets the current system time.
fn do_gettime(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    buffer_addr: VirtualAddress,
) -> Result<(), Error> {
    trace!("do_gettime(): pid={:?}, buffer_addr={:?}", pid, buffer_addr);

    // Get system time.
    let now: SystemTime = clock::now();

    // Copy system time to user space.
    pm.vmcopy_to_user(
        pid,
        buffer_addr,
        VirtualAddress::new(&now as *const SystemTime as usize),
        ::core::mem::size_of::<SystemTime>(),
    )
}

///
/// # Description
///
/// Gets the current system time.
///
/// # Parameters
///
/// - `pm`: A mutable reference to the process manager.
/// - `args.argo0`: The address of the buffer where the system time should be stored.
///
/// # Returns
///
/// If susccessful, `gettime` returns `KcallResult:Success`. Otherwise it returns a
/// `KcallResult::Error` to indicate the error.
///
pub fn gettime(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    // Unpack kernel call arguments.
    let buffer_addr: VirtualAddress = VirtualAddress::new(args.arg0 as usize);

    // Get system time and parse result.
    match do_gettime(pm, args.pid, buffer_addr) {
        Ok(()) => KcallResult::ok(),
        Err(error) => KcallResult::Error(error.code.into()),
    }
}
