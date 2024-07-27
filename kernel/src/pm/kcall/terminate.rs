// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallArgs,
    pm::ProcessManager,
};
use ::sys::pm::ProcessIdentifier;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn terminate(pm: &mut ProcessManager, args: &KcallArgs) -> i32 {
    match pm.terminate(ProcessIdentifier::from(args.arg0 as usize)) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
