// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::unistd::_exit;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn abort() -> ! {
    ::syslog::trace!("abort(): terminating process abnormally");
    
    match _exit(134) {
        Ok(never) => never,
        Err(error) => {
            panic!("abort(): failed to exit ({error:?})");
        }
    }
} 