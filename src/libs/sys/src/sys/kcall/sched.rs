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
    kcall0,
    number::KcallNumber,
};
//==================================================================================================
// Scheduler Yield
//==================================================================================================

pub fn sched_yield() -> Result<(), Error> {
    let result: i64 = kcall0!(KcallNumber::SchedulerYield.into());

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to sched_yield()"))
    }
}
