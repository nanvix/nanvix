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

pub fn resume(eventinfo: usize) -> i32 {
    let eventinfo: EventDescriptor = match EventDescriptor::try_from(eventinfo) {
        Ok(eventinfo) => eventinfo,
        Err(e) => return e.code.into_errno(),
    };

    match EventManager::resume(eventinfo) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
