// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::{
        KcallArgs,
        KcallResult,
    },
    pm::ProcessManager,
};
use ::sys::pm::ProcessIdentifier;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn terminate(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    match pm.terminate(ProcessIdentifier::from(args.arg0)) {
        Ok(()) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
