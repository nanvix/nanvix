// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallResult,
    pm::{
        clock,
        ProcessManager,
    },
};
use ::sys::{
    error::Error,
    mm::VirtualAddress,
    pm::ProcessIdentifier,
    time::SystemTime,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Gets the current system time.
fn do_gettime(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    buffer_addr: VirtualAddress,
) -> Result<(), Error> {
    trace!("pid={pid:?}, buffer_addr={buffer_addr:?}");

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
/// - `pid`: Identifier of the calling process.
/// - `arg0`: The address of the buffer where the system time should be stored.
///
/// # Returns
///
/// If successful, `gettime` returns `KcallResult::Success`. Otherwise it returns a
/// `KcallResult::Error` to indicate the error.
///
pub fn gettime(pid: ProcessIdentifier, arg0: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    // Unpack kernel call arguments.
    let buffer_addr: VirtualAddress = VirtualAddress::new(arg0 as usize);

    // Get system time and parse result.
    match do_gettime(pm, pid, buffer_addr) {
        Ok(()) => KcallResult::ok(),
        Err(error) => KcallResult::Error(error.code.into()),
    }
}
