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
    kcall::KcallArgs,
    pm::ProcessManager,
};
use ::kcall::{
    Error,
    Event,
    EventCtrlRequest,
    ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_evctrl(
    pid: ProcessIdentifier,
    ev: Event,
    req: EventCtrlRequest,
) -> Result<Option<EventOwnership>, Error> {
    EventManager::evctrl(pid, ev, req)
}

pub fn evctrl(pm: &mut ProcessManager, args: &KcallArgs) -> i32 {
    trace!("evctrl(): ev={:?}, req={:?}", args.arg0, args.arg1);

    let ev: Event = match Event::try_from(args.arg0) {
        Ok(ev) => ev,
        Err(e) => return e.code.into_errno(),
    };

    let req: EventCtrlRequest = match EventCtrlRequest::try_from(args.arg1) {
        Ok(req) => req,
        Err(e) => return e.code.into_errno(),
    };

    match do_evctrl(args.pid, ev, req) {
        Ok(Some(ownership)) => match pm.add_event(ownership) {
            Ok(_) => 0,
            Err(e) => e.code.into_errno(),
        },
        Ok(None) => match pm.remove_event(&ev) {
            Ok(_) => 0,
            Err(e) => e.code.into_errno(),
        },
        Err(e) => e.code.into_errno(),
    }
}
