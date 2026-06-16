// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::WorkerThreadError,
    syscalls::{
        SyscallAction,
        SyscallTable,
    },
};
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageType,
    },
    pm::ThreadIdentifier,
};
use ::sysapi::sys_times::tms;
use ::syscall::sys::times::message::{
    TimesRequest,
    TimesResponse,
};

//==================================================================================================
// do_times()
//==================================================================================================

pub fn do_times<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    _request: TimesRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("times(): tid={tid:?}");

    let mut libc_buffer: libc::tms = libc::tms {
        tms_utime: 0,
        tms_stime: 0,
        tms_cutime: 0,
        tms_cstime: 0,
    };

    debug!("libc::times(): buffer={:p}", &libc_buffer as *const libc::tms);

    match unsafe { handle_times(syscall_table, &mut libc_buffer as *mut libc::tms) } {
        -1 => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_times(): worker thread interrupted while blocked on times()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::clock_getres(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_times(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(crate::build_error(tid, error))
        },
        elapsed => {
            debug!(
                "libc::times(): elapsed={:?}, tms.utime={:?}, tms.stime={:?}, tms.cutime={:?}, \
                 tms.cstime={:?}",
                elapsed,
                libc_buffer.tms_utime,
                libc_buffer.tms_stime,
                libc_buffer.tms_cutime,
                libc_buffer.tms_cstime
            );

            let nanvix_buffer: tms = tms {
                tms_utime: libc_buffer.tms_utime,
                tms_stime: libc_buffer.tms_stime,
                tms_cutime: libc_buffer.tms_cutime,
                tms_cstime: libc_buffer.tms_cstime,
            };

            Ok(TimesResponse::build(
                tid,
                elapsed,
                nanvix_buffer,
                ::syscall::LINUXD,
                MessageType::Ikc,
            ))
        },
    }
}

//==================================================================================================
// System Call Wrappers
//==================================================================================================

/// Handler for `libc::times()`.
unsafe fn handle_times<T>(syscall_table: &SyscallTable<T>, buf: *mut libc::tms) -> libc::clock_t {
    match &syscall_table.times {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe { syscall_fn(&syscall_table.state, buf) },
    }
}
