// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::{
        c_int,
        c_void,
    },
    pthread::{
        pthread_attr_t,
        pthread_t,
        syscall,
    },
    sched::sched_param,
    sys::types::size_t,
};
use ::nvx::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Modules
//==================================================================================================

/// Mutexes.
pub mod mutex;

/// Condition variables.
pub mod cond;

/// Thread-specific data area.
pub mod tda;

//==================================================================================================
// pthread_attr_destroy()
//==================================================================================================

///
/// # Description
///
/// Destroys a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
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
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_destroy(attr: *mut pthread_attr_t) -> c_int {
    ::nvx::trace!("pthread_attr_destroy(): attr={:?}", attr);

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::nvx::error!("pthread_attr_destroy(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    (*attr).is_initialized = 0;

    0
}

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
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getdetachstate(
    attr: *const pthread_attr_t,
    detachstate: *mut c_int,
) -> c_int {
    ::nvx::trace!("pthread_attr_getdetachstate(): attr={:?}, detachstate={:?}", attr, detachstate);

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::nvx::error!("pthread_attr_getdetachstate(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `detachstate` is not valid.
    if detachstate.is_null() {
        ::nvx::error!("pthread_attr_getdetachstate(): invalid detach state pointer");
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
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getguardsize(
    attr: *const pthread_attr_t,
    guardsize: *mut size_t,
) -> c_int {
    ::nvx::trace!("pthread_attr_getguardsize(): attr={:?}, guardsize={:?}", attr, guardsize);

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::nvx::error!("pthread_attr_getguardsize(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `guardsize` is not valid.
    if guardsize.is_null() {
        ::nvx::error!("pthread_attr_getguardsize(): invalid guard size pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::nvx::warn!("pthread_attr_getguardsize(): not supported, failing");

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
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getschedparam(
    attr: *const pthread_attr_t,
    param: *mut sched_param,
) -> c_int {
    ::nvx::trace!("pthread_attr_getschedparam(): attr={:?}, param={:?}", attr, param);

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::nvx::error!("pthread_attr_getschedparam(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `param` is not valid.
    if param.is_null() {
        ::nvx::error!("pthread_attr_getschedparam(): invalid sched param pointer");
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
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getstackaddr(
    attr: *const pthread_attr_t,
    stackaddr: *mut *mut c_void,
) -> c_int {
    ::nvx::trace!("pthread_attr_getstackaddr(): attr={:?}, stackaddr={:?}", attr, stackaddr);

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::nvx::error!("pthread_attr_getstackaddr(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `stackaddr` is not valid.
    if stackaddr.is_null() {
        ::nvx::error!("pthread_attr_getstackaddr(): invalid stack address pointer");
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
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getstacksize(
    attr: *const pthread_attr_t,
    stacksize: *mut size_t,
) -> c_int {
    ::nvx::trace!("pthread_attr_getstacksize(): attr={:?}, stacksize={:?}", attr, stacksize);

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::nvx::error!("pthread_attr_getstacksize(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `stacksize` is not valid.
    if stacksize.is_null() {
        ::nvx::error!("pthread_attr_getstacksize(): invalid stack size pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Store the stack size.
    *stacksize = (*attr).stacksize;

    0
}

//==================================================================================================
// pthread_attr_getstack()
//==================================================================================================

///
/// # Description
///
/// Gets the stack address and size attributes in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
/// - `stackaddr`: Storage location for the stack address.
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
/// - `stackaddr` points to a valid `c_void` pointer.
/// - `stacksize` points to a valid `size_t` variable.
///
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getstack(
    attr: *const pthread_attr_t,
    stackaddr: *mut *mut c_void,
    stacksize: *mut size_t,
) -> c_int {
    ::nvx::trace!(
        "pthread_attr_getstack(): attr={:?}, stackaddr={:?}, stacksize={:?}",
        attr,
        stackaddr,
        stacksize
    );

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::nvx::error!("pthread_attr_getstack(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `stackaddr` is not valid.
    if stackaddr.is_null() {
        ::nvx::error!("pthread_attr_getstack(): invalid stack address pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `stacksize` is not valid.
    if stacksize.is_null() {
        ::nvx::error!("pthread_attr_getstack(): invalid stack size pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Store the stack address and size.
    *stackaddr = (*attr).stackaddr;
    *stacksize = (*attr).stacksize;

    0
}

//==================================================================================================
// pthread_attr_init()
//==================================================================================================

///
/// # Description
///
/// Initializes a thread attributes object with default values.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
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
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_init(attr: *mut pthread_attr_t) -> c_int {
    ::nvx::trace!("pthread_attr_init(): attr={:?}", attr);

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::nvx::error!("pthread_attr_init(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    *attr = pthread_attr_t::default();

    0
}

//==================================================================================================
// pthread_create()
//==================================================================================================

///
/// # Description
///
/// Creates a new thread.
///
/// # Parameters
///
/// - `thread`: Thread identifier.
/// - `attr`: Thread attributes.
/// - `start_routine`: Thread function.
/// - `arg`: Argument passed to the thread function.
///
/// # Returns
///
/// If successful, zero is returned. Otherwise, an error code is returned instead.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is call to safe this function if the following conditions are met:
///
/// - `thread` points to a valid `pthread_t` structure.
/// - If `attr` is not null, it points to a valid `pthread_attr_t` structure.
/// - `start_routine` is a valid function pointer.
///
#[no_mangle]
pub unsafe extern "C" fn pthread_create(
    thread: *mut pthread_t,
    attr: *const pthread_attr_t,
    start_routine: extern "C" fn(usize) -> usize,
    arg: usize,
) -> c_int {
    ::nvx::trace!(
        "pthread_create(): thread={:?}, attr={:?}, start_routine={:#x?}, arg={:#x?}",
        thread,
        attr,
        start_routine as usize,
        arg
    );

    // Check if `thread` is not valid.
    if thread.is_null() {
        ::nvx::error!("pthread_create(): invalid thread pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Cast `thread` to a mutable reference.
    let thread: &mut pthread_t = &mut *thread;

    // Check if we should use default attributes.
    if attr.is_null() {
        // TODO: use default attributes.
    } else {
        ::nvx::warn!("pthread_create(): attributes are not supported, ignoring");
    }

    match syscall::pthread_create(start_routine, arg) {
        Ok(tid) => {
            *thread = tid;
            0
        },
        Err(error) => error.code.get(),
    }
}

//==================================================================================================
// pthread_detach()
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub extern "C" fn pthread_detach(_thread: pthread_t) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/502
    ::nvx::error!("pthread_detach(): not implemented");
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
#[no_mangle]
pub extern "C" fn pthread_exit(retval: *mut c_void) -> ! {
    let error: Error = syscall::pthread_exit(retval as usize).unwrap_err();
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
#[no_mangle]
pub extern "C" fn pthread_equal(thread1: pthread_t, thread2: pthread_t) -> c_int {
    ::nvx::trace!("pthread_equal(): thread1={:?}, thread2={:?}", thread1, thread2);

    if thread1 == thread2 {
        1
    } else {
        0
    }
}

//==================================================================================================
// pthread_join()
//==================================================================================================

///
/// # Description
///
/// Waits for a thread to terminate.
///
/// # Parameters
///
/// - `thread`: Thread identifier.
/// - `retval_ptr`: Pointer to the location where the return value of the thread will be stored.
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
/// - If `retval_ptr` is not null, it points to a valid pointer.
///
#[no_mangle]
pub unsafe extern "C" fn pthread_join(thread: pthread_t, retval_ptr: *mut *mut c_void) -> c_int {
    ::nvx::trace!(
        "pthread_join(): _thread={:?}, retval_ptr={:?}, *retval={:?}",
        thread,
        retval_ptr,
        *retval_ptr
    );

    match syscall::pthread_join(thread) {
        Ok(retval) => {
            nvx::trace!("pthread_join(): retval={:#x?}", retval);
            if !retval_ptr.is_null() {
                *retval_ptr = retval as *mut c_void;
            }
            0
        },
        Err(error) => error.code.get(),
    }
}

//==================================================================================================
// pthread_self()
//==================================================================================================

///
/// # Description
///
/// Returns the thread identifier of the calling thread.
///
/// # Returns
///
/// The thread identifier of the calling thread.
///
#[no_mangle]
pub extern "C" fn pthread_self() -> pthread_t {
    syscall::pthread_self()
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
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setdetachstate(
    attr: *mut pthread_attr_t,
    detachstate: c_int,
) -> c_int {
    ::nvx::trace!("pthread_attr_setdetachstate(): attr={:?}, detachstate={:?}", attr, detachstate);

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::nvx::error!("pthread_attr_setdetachstate(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::nvx::warn!("pthread_attr_setdetachstate(): not supported, failing");
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
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setguardsize(
    attr: *mut pthread_attr_t,
    guardsize: size_t,
) -> c_int {
    ::nvx::trace!("pthread_attr_setguardsize(): attr={:?}, guardsize={:?}", attr, guardsize);

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::nvx::error!("pthread_attr_setguardsize(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::nvx::warn!("pthread_attr_setguardsize(): not supported, failing");
    ErrorCode::OperationNotSupported.get()
}

//==================================================================================================
// pthread_attr_setschedparam()
//==================================================================================================

///
/// # Description
///
/// Sets the scheduling parameters of a thread.
///
/// # Parameters
///
/// - `thread`: Thread identifier.
/// - `policy`: Scheduling policy.
/// - `param`: Scheduling parameters.
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
/// - `param` points to a valid `sched_param` structure.
///
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setschedparam(
    thread: pthread_t,
    policy: c_int,
    param: *const sched_param,
) -> c_int {
    ::nvx::trace!(
        "pthread_attr_setschedparam(): thread={:?}, policy={}, param={:?}",
        thread,
        policy,
        param
    );

    // Check if `param` is not valid.
    if param.is_null() {
        ::nvx::error!("pthread_attr_setschedparam(): invalid sched param pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::nvx::warn!("pthread_attr_setschedparam(): not supported, failing");
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
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setstack(
    attr: *mut pthread_attr_t,
    stackaddr: *mut c_void,
    stacksize: size_t,
) -> c_int {
    ::nvx::trace!(
        "pthread_attr_setstack(): attr={:?}, stackaddr={:?}, stacksize={:?}",
        attr,
        stackaddr,
        stacksize
    );

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::nvx::error!("pthread_attr_setstack(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::nvx::warn!("pthread_attr_setstack(): not supported, failing");
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
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setstackaddr(
    attr: *mut pthread_attr_t,
    stackaddr: *mut c_void,
) -> c_int {
    ::nvx::trace!("pthread_attr_setstackaddr(): attr={:?}, stackaddr={:?}", attr, stackaddr);

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::nvx::error!("pthread_attr_setstackaddr(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::nvx::warn!("pthread_attr_setstackaddr(): not supported, failing");
    ErrorCode::OperationNotSupported.get()
}

//==================================================================================================
// pthread_attr_setstacksize()
//==================================================================================================

///
/// # Description
///
/// Sets the stack size attribute in a thread attributes object.
///
/// # Parameters
///
/// - `attr`: Thread attributes object.
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
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setstacksize(
    attr: *mut pthread_attr_t,
    stacksize: size_t,
) -> c_int {
    ::nvx::trace!("pthread_attr_setstacksize(): attr={:?}, stacksize={:?}", attr, stacksize);

    // Check if `attr` is not valid.
    if attr.is_null() {
        ::nvx::error!("pthread_attr_setstacksize(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::nvx::warn!("pthread_attr_setstacksize(): not supported, failing");
    ErrorCode::OperationNotSupported.get()
}

//==================================================================================================
// pthread_setcancelstate()
//==================================================================================================

///
/// # Description
///
/// Sets the cancelability state of the calling thread.
///
/// # Parameters
///
/// - `state`: New cancelability state.
/// - `oldstate`: Old cancelability state.
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
/// - `oldstate` points to a valid `c_int` variable.
///
#[no_mangle]
pub unsafe extern "C" fn pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int {

    // Check if `oldstate` is not valid.
    if !oldstate.is_null() {
        ::nvx::error!("pthread_setcancelstate(): invalid old state pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::nvx::warn!("pthread_setcancelstate(): not supported, ignoring");
    0
}

//==================================================================================================
// pthread_setcanceltype()
//==================================================================================================

///
/// # Description
///
/// Sets the cancelability type of the calling thread.
///
/// # Parameters
///
/// - `type_`: New cancelability type.
/// - `oldtype`: Old cancelability type.
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
#[no_mangle]
pub unsafe extern "C" fn pthread_setcanceltype(type_: c_int, oldtype: *mut c_int) -> c_int {

    // Check if `oldtype` is not valid.
    if !oldtype.is_null() {
        ::nvx::error!("pthread_setcanceltype(): invalid old type pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::nvx::warn!("pthread_setcanceltype(): not supported, failing");
    ErrorCode::OperationNotSupported.get()
}
