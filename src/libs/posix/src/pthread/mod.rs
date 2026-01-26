// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sched::sched_param,
    sys_types::{
        c_size_t,
        pthread_attr_t,
        pthread_once_t,
        pthread_t,
    },
};
use ::syscall::pthread::{
    self,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Modules
//==================================================================================================

/// Mutexes.
pub mod mutex;

/// Thread-specific data area.
pub mod tda;

//==================================================================================================
// pthread_attr_getdetachstate()
//==================================================================================================

///
/// # Description
///
/// Gets the detach state attribute in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
/// - `detachstate`: Storage location for the detach state.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `attr` points to a valid `pthread_attr_t` structure.
/// - `detachstate` points to a valid `c_int` variable.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_attr_getdetachstate(
    attr: *const pthread_attr_t,
    detachstate: *mut c_int,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::error!("pthread_attr_getdetachstate(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `detachstate` is not valid.
    if detachstate.is_null() {
        ::syslog::error!("pthread_attr_getdetachstate(): invalid detach state pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Store the detach state.
    *detachstate = (*attr).detachstate;

    0
}

//==================================================================================================
// pthread_attr_getguardsize()
//==================================================================================================

///
/// # Description
///
/// Gets the guard size attribute in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
/// - `guardsize`: Storage location for the guard size.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `attr` points to a valid `pthread_attr_t` structure.
/// - `guardsize` points to a valid `size_t` variable.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_attr_getguardsize(
    attr: *const pthread_attr_t,
    guardsize: *mut c_size_t,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::error!("pthread_attr_getguardsize(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `guardsize` is not valid.
    if guardsize.is_null() {
        ::syslog::error!("pthread_attr_getguardsize(): invalid guard size pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::syslog::warn!("pthread_attr_getguardsize(): not supported, failing");

    ErrorCode::OperationNotSupported.get()
}

//==================================================================================================
// pthread_attr_getschedparam()
//==================================================================================================

///
/// # Description
///
/// Gets the scheduling parameter attribute in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
/// - `param`: Storage location for the scheduling parameter.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `attr` points to a valid `pthread_attr_t` structure.
/// - `param` points to a valid `sched_param` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_attr_getschedparam(
    attr: *const pthread_attr_t,
    param: *mut sched_param,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::error!("pthread_attr_getschedparam(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `param` is not valid.
    if param.is_null() {
        ::syslog::error!("pthread_attr_getschedparam(): invalid sched param pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Store the scheduling parameter.
    *param = (*attr).schedparam;

    0
}

//==================================================================================================
// pthread_attr_getstackaddr()
//==================================================================================================

///
/// # Description
///
/// Gets the stack address attribute in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
/// - `stackaddr`: Storage location for the stack address.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `attr` points to a valid `pthread_attr_t` structure.
/// - `stackaddr` points to a valid `*mut c_void` variable.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_attr_getstackaddr(
    attr: *const pthread_attr_t,
    stackaddr: *mut *mut c_void,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::error!("pthread_attr_getstackaddr(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `stackaddr` is not valid.
    if stackaddr.is_null() {
        ::syslog::error!("pthread_attr_getstackaddr(): invalid stack address pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Store the stack address.
    *stackaddr = (*attr).stackaddr;

    0
}

//==================================================================================================
// pthread_attr_getstacksize()
//==================================================================================================

///
/// # Description
///
/// Gets the stack size attribute in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
/// - `stacksize`: Storage location for the stack size.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is call to safe this function if the following conditions are met:
///
/// - `attr` points to a valid `pthread_attr_t` structure.
/// - `stacksize` points to a valid `size_t` variable.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_attr_getstacksize(
    attr: *const pthread_attr_t,
    stacksize: *mut c_size_t,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::error!("pthread_attr_getstacksize(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `stacksize` is not valid.
    if stacksize.is_null() {
        ::syslog::error!("pthread_attr_getstacksize(): invalid stack size pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Store the stack size.
    *stacksize = (*attr).stacksize;

    0
}

//==================================================================================================
// pthread_detach()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub extern "C" fn pthread_detach(_thread: pthread_t) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/502
    ::syslog::debug!("pthread_detach(): not implemented");
    ErrorCode::OperationNotSupported.get()
}

//==================================================================================================
// pthread_exit()
//==================================================================================================

///
/// # Description
///
/// Terminates the calling thread.
///
/// # Parameters
///
/// - `retval`: Return value of the thread.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub extern "C" fn pthread_exit(retval: *mut c_void) -> ! {
    let error: Error = pthread::pthread_exit(retval as usize).unwrap_err();
    panic!("pthread_exit(): {:?}", error);
}

//==================================================================================================
// pthread_equal()
//==================================================================================================

///
/// # Description
///
/// Compares two thread identifiers.
///
/// # Parameters
///
/// - `thread1`: First thread identifier.
/// - `thread2`: Second thread identifier.
///
/// # Returns
///
/// On success, a non-zero value is returned if the two thread identifiers are equal, and zero otherwise.
/// If either t1 or t2 is not a valid thread ID and is not equal to `PTHREAD_NULL`, the behavior is undefined.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub extern "C" fn pthread_equal(thread1: pthread_t, thread2: pthread_t) -> c_int {
    if thread1 == thread2 {
        1
    } else {
        0
    }
}

//==================================================================================================
// pthread_once()
//==================================================================================================

///
/// # Description
///
/// Calls the specified initialization function exactly once, even if called from multiple threads.
///
/// # Parameters
///
/// - `once_control`: Pointer to a control variable that determines whether the initialization function has been called.
/// - `init_routine`: Pointer to the initialization function to be called.
///
/// # Returns
///
/// The `pthread_once()` function always returns `0` on success. On error, it returns an error number.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and call a function pointer.
///
/// It is safe to call this function if the following conditions are met:
/// - `once_control` points to a valid `pthread_once_t` object.
/// - `init_routine` is a valid function pointer.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_once(
    once_control: *mut pthread_once_t,
    init_routine: Option<unsafe extern "C" fn()>,
) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/513
    ::syslog::debug!("pthread_once(): not implemented");
    0
}

//==================================================================================================
// pthread_attr_setdetachstate()
//==================================================================================================

///
/// # Description
///
/// Sets the detach state attribute in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
/// - `detachstate`: New detach state.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `attr` points to a valid `pthread_attr_t` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_attr_setdetachstate(
    attr: *mut pthread_attr_t,
    detachstate: c_int,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::error!("pthread_attr_setdetachstate(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::syslog::warn!("pthread_attr_setdetachstate(): not supported, failing");
    ErrorCode::OperationNotSupported.get()
}

//==================================================================================================
// pthread_attr_setguardsize()
//==================================================================================================

///
/// # Description
///
/// Sets the guard size attribute in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
/// - `guardsize`: New guard size.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `attr` points to a valid `pthread_attr_t` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_attr_setguardsize(
    attr: *mut pthread_attr_t,
    guardsize: c_size_t,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::error!("pthread_attr_setguardsize(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::syslog::warn!("pthread_attr_setguardsize(): not supported, failing");
    ErrorCode::OperationNotSupported.get()
}

//==================================================================================================
// pthread_attr_setschedparam()
//==================================================================================================

///
/// # Description
///
/// Sets the scheduling parameters stored in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object to update.
/// - `param`: Scheduling parameters to store in `attr`.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `attr` points to a valid `pthread_attr_t` structure.
/// - `param` points to a valid `sched_param` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_attr_setschedparam(
    attr: *mut pthread_attr_t,
    param: *const sched_param,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::error!("pthread_attr_setschedparam(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `param` is not valid.
    if param.is_null() {
        ::syslog::error!("pthread_attr_setschedparam(): invalid sched param pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::syslog::warn!("pthread_attr_setschedparam(): not supported, failing");
    ErrorCode::OperationNotSupported.get()
}

//==================================================================================================
// pthread_attr_setstack()
//==================================================================================================

///
/// # Description
///
/// Sets the stack address and size attributes in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
/// - `stackaddr`: New stack address.
/// - `stacksize`: New stack size.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `attr` points to a valid `pthread_attr_t` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_attr_setstack(
    attr: *mut pthread_attr_t,
    stackaddr: *mut c_void,
    stacksize: c_size_t,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::error!("pthread_attr_setstack(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::syslog::warn!("pthread_attr_setstack(): not supported, failing");
    ErrorCode::OperationNotSupported.get()
}

//==================================================================================================
// pthread_attr_setstackaddr()
//==================================================================================================

///
/// # Description
///
/// Sets the stack address attribute in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
/// - `stackaddr`: New stack address.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `attr` points to a valid `pthread_attr_t` structure.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_attr_setstackaddr(
    attr: *mut pthread_attr_t,
    stackaddr: *mut c_void,
) -> c_int {
    // Check if `attr` is not valid.
    if attr.is_null() {
        ::syslog::error!("pthread_attr_setstackaddr(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::syslog::warn!("pthread_attr_setstackaddr(): not supported, failing");
    ErrorCode::OperationNotSupported.get()
}

//==================================================================================================
// pthread_setcanceltype()
//==================================================================================================

///
/// # Description
///
/// Sets the cancellability type of the calling thread.
///
/// # Parameters
///
/// - `type_`: New cancellability type.
/// - `oldtype`: Old cancellability type.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `oldtype` points to a valid `c_int` variable.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_setcanceltype(_type_: c_int, oldtype: *mut c_int) -> c_int {
    // Check if `oldtype` is not valid.
    if oldtype.is_null() {
        ::syslog::error!("pthread_setcanceltype(): invalid old type pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::syslog::warn!("pthread_setcanceltype(): not supported, failing");
    ErrorCode::OperationNotSupported.get()
}
