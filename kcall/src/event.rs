// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    number::KcallNumber,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use ::sys::event::*;

//==================================================================================================
// Wait Event
//==================================================================================================

pub fn wait(info: &mut EventInformation) -> Result<(), Error> {
    let result: i32 = unsafe {
        crate::arch::kcall1(KcallNumber::Wait.into(), info as *mut EventInformation as u32)
    };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to wait()"))
    }
}

//==================================================================================================
// Resume Event
//==================================================================================================

pub fn resume(event: EventDescriptor) -> Result<(), Error> {
    let result: i32 =
        unsafe { crate::arch::kcall1(KcallNumber::Resume.into(), usize::from(event) as u32) };

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
    let result: i32 = unsafe {
        crate::arch::kcall2(KcallNumber::EventCtrl.into(), u32::from(ev), u32::from(req))
    };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to evctrl()"))
    }
}
