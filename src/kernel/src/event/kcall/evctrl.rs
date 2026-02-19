// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    event::manager::{
        EventManager,
        EventOwnership,
    },
    kcall::KcallResult,
    pm::ProcessManager,
};
use ::sys::{
    error::Error,
    event::{
        Event,
        EventCtrlRequest,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_evctrl(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    ev: Event,
    req: EventCtrlRequest,
) -> Result<Option<EventOwnership>, Error> {
    EventManager::evctrl(pm, pid, ev, req)
}

///
/// # Description
///
/// Kernel call handler for controlling event ownership.
///
/// # Parameters
///
/// - `pid`: Identifier of the calling process.
/// - `arg0`: Encoded event identifier.
/// - `arg1`: Encoded event control request.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn evctrl(pid: ProcessIdentifier, arg0: u32, arg1: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    trace!("ev={:?}, req={:?}", arg0, arg1);

    let ev: Event = match Event::try_from(arg0) {
        Ok(ev) => ev,
        Err(e) => return KcallResult::Error(e.code.into()),
    };

    let req: EventCtrlRequest = match EventCtrlRequest::try_from(arg1) {
        Ok(req) => req,
        Err(e) => return KcallResult::Error(e.code.into()),
    };

    match do_evctrl(pm, pid, ev, req) {
        Ok(Some(ownership)) => match pm.add_event(ownership) {
            Ok(_) => KcallResult::ok(),
            Err(e) => KcallResult::Error(e.code.into()),
        },
        Ok(None) => match pm.remove_event(&ev) {
            Ok(_) => KcallResult::ok(),
            Err(e) => KcallResult::Error(e.code.into()),
        },
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
