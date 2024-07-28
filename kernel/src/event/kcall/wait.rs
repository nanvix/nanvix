// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    event::manager::EventManager,
    pm::ProcessManager,
};
use ::sys::{
    event::EventInformation,
    mm::{
        Address,
        VirtualAddress,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn wait(info: *mut EventInformation, interrupts: usize, exceptions: usize) -> i32 {
    let pid: ProcessIdentifier = match ProcessManager::get_pid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };
    match EventManager::wait(interrupts, exceptions) {
        Ok(message) => {
            // Copy message payload to event
            let dst: VirtualAddress = match VirtualAddress::from_raw_value(info as usize) {
                Ok(dst) => dst,
                Err(e) => return e.code.into_errno(),
            };

            let src: VirtualAddress = match VirtualAddress::from_raw_value(
                message.payload.as_ref().as_ptr() as *const u8 as usize,
            ) {
                Ok(src) => src,
                Err(e) => return e.code.into_errno(),
            };

            let size: usize = core::mem::size_of::<EventInformation>();

            if let Err(e) = ProcessManager::vmcopy_to_user(pid, dst, src, size) {
                return e.code.into_errno();
            }
            0
        },
        Err(e) => e.code.into_errno(),
    }
}
