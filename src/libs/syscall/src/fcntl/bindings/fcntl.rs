// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    safe::{
        file,
        FileControlRequest,
    },
};
use ::sysapi::ffi::c_int;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[trace_syscall]
#[no_mangle]
pub unsafe extern "C" fn fcntl(fd: c_int, cmd: c_int, args: ...) -> c_int {
    // Attempt to convert the command and arguments.
    let cmd: FileControlRequest = match FileControlRequest::try_from((cmd, args)) {
        Ok(cmd) => cmd,
        Err(error) => {
            ::syslog::error!("fcntl(): invalid command ({error:?}, fd={fd:?}, cmd={cmd:?})");
            *__errno_location() = error.code.get();
            return -1;
        },
    };

    match file::fcntl(fd, &cmd) {
        Ok(ret) => ret,
        Err(error) => {
            ::syslog::error!("fcntl(): failed ({error:?}, fd={fd:?}, cmd={cmd:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
