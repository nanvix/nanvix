// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::unistd;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn test_getpid() {
    // Try to get PID
    match unistd::getpid() {
        Ok(pid) => {
            ::syslog::info!("got PID {:#?}", pid);
        },
        Err(err) => {
            panic!("failed to get PID: {:?}", err);
        },
    };
}

pub fn test() {
    test_getpid();
}
