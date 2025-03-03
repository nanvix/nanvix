// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::event::manager::EventManager;
use ::sys::event::EventDescriptor;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Resumes an execution after an event.
///
/// # Parameters
///
/// - `evdesc`: Event descriptor.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
/// # Safety
///
/// This function is unsafe because it operates on global variables.
///
/// This function is safe to use if and only if the following conditions are met:
///
/// - The calling process does not hold a reference to the process manager.
///
pub unsafe fn resume(evdesc: usize) -> i32 {
    let eventinfo: EventDescriptor = match EventDescriptor::try_from(evdesc) {
        Ok(eventinfo) => eventinfo,
        Err(e) => return e.code.into_errno(),
    };

    match EventManager::resume(eventinfo) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
