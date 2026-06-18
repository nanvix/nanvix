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
use ::core::{
    cmp,
    ptr,
};
use ::log::{
    debug,
    error,
    trace,
};
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageType,
    },
    pm::ThreadIdentifier,
};
use ::sysapi::sys_select::{
    fd_set,
    timeval,
    FdSetError,
    FD_SETSIZE,
};
use ::syscall::sys::select::message::{
    SelectRequest,
    SelectResponse,
};

//==================================================================================================
// Static Asserts
//==================================================================================================

// Ensure safe conversions between Nanvix and libc fd_set structures.
::static_assert::assert_eq!(FD_SETSIZE <= libc::FD_SETSIZE);

//==================================================================================================
// do_select()
//==================================================================================================

pub fn do_select<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: SelectRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("select(): tid={tid:?}, request={request:?}");
    // Validate request.
    if request.nfds == 0 || request.nfds as usize > FD_SETSIZE {
        error!("select(): invalid nfds (nfds={:?})", request.nfds);
        return Ok(crate::build_error(tid, ErrorCode::InvalidArgument));
    }

    // Prepare timeout.
    let mut timeout_storage: libc::timeval = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let timeout_ptr: *mut libc::timeval = if let Some(bytes) = request.timeout {
        let tv: timeval = match timeval::try_from_bytes(&bytes) {
            Ok(tv) => tv,
            Err(e) => {
                error!("select(): invalid timeout payload (error={e:?})");
                return Ok(crate::build_error(tid, ErrorCode::InvalidMessage));
            },
        };
        timeout_storage.tv_sec = tv.tv_sec as libc::time_t;
        timeout_storage.tv_usec = tv.tv_usec as libc::suseconds_t;
        &mut timeout_storage
    } else {
        ptr::null_mut()
    };

    let nfds_c_int: libc::c_int = cmp::min(request.nfds as usize, FD_SETSIZE) as libc::c_int; // safe due to validation.

    let mut read_ptr: Option<LibcFdSet> = request
        .readfds
        .as_ref()
        .map(|fd| fd.try_into())
        .transpose()?;
    let mut write_ptr: Option<LibcFdSet> = request
        .writefds
        .as_ref()
        .map(|fd| fd.try_into())
        .transpose()?;
    let mut error_ptr: Option<LibcFdSet> = request
        .errorfds
        .as_ref()
        .map(|fd| fd.try_into())
        .transpose()?;

    debug!(
        "libc::select(): nfds={nfds_c_int:?}, read_ptr={:?}, write_ptr={:?}, error_ptr={:?}, \
         timeout_ptr={:?}",
        read_ptr, write_ptr, error_ptr, timeout_ptr
    );

    // SAFETY: All pointers either null or point to valid local storage; nfds validated.
    let nready: libc::c_int = unsafe {
        handle_select(
            syscall_table,
            nfds_c_int,
            read_ptr
                .as_mut()
                .map_or(ptr::null_mut(), |r| &mut r.0 as *mut libc::fd_set),
            write_ptr
                .as_mut()
                .map_or(ptr::null_mut(), |w| &mut w.0 as *mut libc::fd_set),
            error_ptr
                .as_mut()
                .map_or(ptr::null_mut(), |e| &mut e.0 as *mut libc::fd_set),
            timeout_ptr,
        )
    };

    if nready >= 0 {
        debug!("select(): nready={nready:?}");
        match u8::try_from(nready) {
            Ok(ready_u8) => {
                let readfds: Option<fd_set> = read_ptr.map(|set| set.try_into()).transpose()?;
                let writefds: Option<fd_set> = write_ptr.map(|set| set.try_into()).transpose()?;
                let errorfds: Option<fd_set> = error_ptr.map(|set| set.try_into()).transpose()?;

                let response: Message = SelectResponse::build(
                    tid,
                    ready_u8,
                    &readfds,
                    &writefds,
                    &errorfds,
                    ::syscall::LINUXD,
                    MessageType::Ikc,
                );
                Ok(response)
            },
            Err(_e) => {
                error!("select(): nready overflow (nready={nready:?})");
                Ok(crate::build_error(tid, ErrorCode::ValueOutOfRange))
            },
        }
    } else {
        let errno: libc::c_int = unsafe { *libc::__errno_location() };
        if errno == libc::EINTR {
            error!("do_select(): worker thread interrupted while blocked on select()");
            return Err(WorkerThreadError::Interrupted);
        }
        error!("select(): errno={errno:?}");
        let code: ErrorCode = ErrorCode::try_from(errno).unwrap_or_else(|_| {
            error!("select(): unknown errno value {errno:?}, returning ErrorCode::TryAgain");
            ErrorCode::TryAgain
        });
        Ok(crate::build_error(tid, code))
    }
}

#[derive(Debug)]
struct LibcFdSet(libc::fd_set);

impl TryFrom<&fd_set> for LibcFdSet {
    type Error = WorkerThreadError;

    fn try_from(value: &fd_set) -> Result<Self, Self::Error> {
        let mut libc_fd_set: libc::fd_set = unsafe { core::mem::zeroed() };

        unsafe {
            libc::FD_ZERO(&mut libc_fd_set as *mut libc::fd_set);
        }

        // Set bits.
        for fd in 0..FD_SETSIZE {
            match value.is_set(fd) {
                Ok(true) => {
                    // SAFETY: pointer is valid.
                    unsafe {
                        libc::FD_SET(fd as libc::c_int, &mut libc_fd_set as *mut libc::fd_set);
                    }
                },
                Ok(false) => {},
                Err(FdSetError::FileDescriptorOutOfRange) => {
                    error!("LibcFdSet::try_from(): fd out of range while reading (fd={fd:?})");
                },
            }
        }

        Ok(Self(libc_fd_set))
    }
}

impl TryFrom<LibcFdSet> for fd_set {
    type Error = WorkerThreadError;

    fn try_from(value: LibcFdSet) -> Result<Self, Self::Error> {
        let mut fd_set: fd_set = fd_set::default();

        // Set bits.
        for fd in 0..FD_SETSIZE {
            // SAFETY: pointer is valid.
            let is_set =
                unsafe { libc::FD_ISSET(fd as libc::c_int, &value.0 as *const libc::fd_set) };
            if is_set {
                if let Err(FdSetError::FileDescriptorOutOfRange) = fd_set.set_bit(fd) {
                    error!("fd_set::try_from(): fd out of range while setting (fd={fd:?})");
                }
            }
        }

        Ok(fd_set)
    }
}

//==================================================================================================
// Wrapper Functions
//==================================================================================================

unsafe fn handle_select<T>(
    syscall_table: &SyscallTable<T>,
    nfds: libc::c_int,
    readfds: *mut libc::fd_set,
    writefds: *mut libc::fd_set,
    errorfds: *mut libc::fd_set,
    timeout: *mut libc::timeval,
) -> libc::c_int {
    match &syscall_table.select {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, nfds, readfds, writefds, errorfds, timeout)
        },
    }
}
