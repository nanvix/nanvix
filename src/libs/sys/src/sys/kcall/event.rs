// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::{
        Error,
        ErrorCode,
    },
    event::{
        Event,
        EventCtrlRequest,
        EventDescriptor,
    },
    kcall1,
    kcall2,
    number::KcallNumber,
};

//==================================================================================================
// Resume Event
//==================================================================================================

pub fn resume(event: EventDescriptor) -> Result<(), Error> {
    let result: i64 = kcall1!(KcallNumber::Resume.into(), usize::from(event) as u32);

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to resume()"))
    }
}

//==================================================================================================
// Controls an Event
//==================================================================================================

pub fn evctrl(ev: Event, req: EventCtrlRequest) -> Result<(), Error> {
    let result: i64 = kcall2!(KcallNumber::EventCtrl.into(), u32::from(ev), u32::from(req));

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to evctrl()"))
    }
}
