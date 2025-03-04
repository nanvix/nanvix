// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::VirtualAddress,
    kcall::KcallArgs,
    pm::ProcessManager,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Unlocks a mutex.
///
/// # Parameters
///
/// - `pm`: Process manager.
/// - `args`: Kernel call arguments.
///
/// # Return
///
/// Upon successful completion, zero is returned. Upon failure, a negative error code is returned
/// instead.
///
pub fn unlock_mutex(pm: &mut ProcessManager, args: &KcallArgs) -> i32 {
    // Unpack kernel call arguments.
    let addr: VirtualAddress = VirtualAddress::from_raw_value(args.arg0 as usize);
    trace!("unlock_mutex(): pid={:?}, tid={:?}, addr={:#x?}", args.pid, args.tid, addr);

    match pm.take_mutex_guard(args.pid, args.tid, addr) {
        Ok(_mutex_guard) => {
            // The mutex guard is dropped, causing threads to be notified.
            0
        },
        Err(e) => e.code.into_errno(),
    }
}
