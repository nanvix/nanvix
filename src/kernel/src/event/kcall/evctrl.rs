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
    kcall::{
        KcallArgs,
        KcallResult,
    },
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

pub fn evctrl(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    trace!("evctrl(): ev={:?}, req={:?}", args.arg0, args.arg1);

    let ev: Event = match Event::try_from(args.arg0) {
        Ok(ev) => ev,
        Err(e) => return KcallResult::Error(e.code.into()),
    };

    let req: EventCtrlRequest = match EventCtrlRequest::try_from(args.arg1) {
        Ok(req) => req,
        Err(e) => return KcallResult::Error(e.code.into()),
    };

    match do_evctrl(pm, args.pid, ev, req) {
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
