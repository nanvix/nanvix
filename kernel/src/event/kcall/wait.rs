// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::event::manager::EventManager;
use ::kcall::EventInformation;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn wait(info: *mut EventInformation, interrupts: usize, exceptions: usize) -> i32 {
    match EventManager::wait(info, interrupts, exceptions) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
