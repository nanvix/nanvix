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
        ::syslog::warn!("pthread_attr_getdetachstate(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `detachstate` is not valid.
    if detachstate.is_null() {
        ::syslog::warn!("pthread_attr_getdetachstate(): invalid detach state pointer");
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
        ::syslog::warn!("pthread_attr_getguardsize(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `guardsize` is not valid.
    if guardsize.is_null() {
        ::syslog::warn!("pthread_attr_getguardsize(): invalid guard size pointer");
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
        ::syslog::warn!("pthread_attr_getschedparam(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `param` is not valid.
    if param.is_null() {
        ::syslog::warn!("pthread_attr_getschedparam(): invalid sched param pointer");
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
        ::syslog::warn!("pthread_attr_getstackaddr(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `stackaddr` is not valid.
    if stackaddr.is_null() {
        ::syslog::warn!("pthread_attr_getstackaddr(): invalid stack address pointer");
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
        ::syslog::warn!("pthread_attr_getstacksize(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `stacksize` is not valid.
    if stacksize.is_null() {
        ::syslog::warn!("pthread_attr_getstacksize(): invalid stack size pointer");
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
pub extern "C" fn pthread_detach(thread: pthread_t) -> c_int {
    match ::sys::pm::ThreadIdentifier::try_from(thread) {
        Ok(tid) => match ::sys::kcall::pm::__kcall_detach_thread(tid) {
            Ok(()) => 0,
            Err(error) => {
                ::syslog::warn!("pthread_detach(): detach_thread failed ({error:?})");
                error.code.get()
            },
        },
        Err(error) => {
            ::syslog::warn!("pthread_detach(): invalid thread id ({error:?})");
            ErrorCode::InvalidArgument.get()
        },
    }
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

/// State constants used internally by `pthread_once()` for the
/// `init_executed` field of `pthread_once_t`.
///
/// # Description
///
/// `init_executed` doubles as a state-machine word.  The four
/// values directly mirror musl libc's encoding in
/// `src/thread/pthread_once.c`, which lets us upgrade to a
/// futex-based multi-threaded implementation without changing
/// the state semantics.
///
/// In a future multi-threaded model:
///
/// - `ONCE_NEVER_RUN` → CAS to `ONCE_IN_PROGRESS` and run `init`.
/// - `ONCE_IN_PROGRESS` → CAS to `ONCE_WAIT` and block on futex.
/// - `ONCE_WAIT` → continue waiting on the futex word.
/// - `ONCE_DONE` → fast-path return.
///
/// In the current single-threaded model only `NEVER_RUN`,
/// `IN_PROGRESS`, and `DONE` are reachable; `ONCE_WAIT` is
/// reserved for the future upgrade.
const ONCE_NEVER_RUN: c_int = 0;
const ONCE_DONE: c_int = 1;
const ONCE_IN_PROGRESS: c_int = 2;
#[allow(dead_code)]
const ONCE_WAIT: c_int = 3;

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
    // Argument validation.
    if once_control.is_null() {
        ::syslog::warn!("pthread_once(): null once_control");
        return ErrorCode::InvalidArgument.get();
    }
    let Some(init_fn) = init_routine else {
        ::syslog::warn!("pthread_once(): null init_routine");
        return ErrorCode::InvalidArgument.get();
    };

    // Sanity check: `is_initialized` must equal 1 after
    // `PTHREAD_ONCE_INIT`.  Any other value indicates a caller
    // bug (e.g. a forgotten or corrupted static initializer).
    //
    // We deliberately do NOT materialise a `&mut pthread_once_t`
    // here: the implementation explicitly supports a recursive
    // call to `pthread_once()` on the same control word from
    // inside `init_routine` (see the IN_PROGRESS branch below),
    // and a recursive call would create a second `&mut` to the
    // same object, which is Stacked-Borrows UB.  All field
    // access goes through `*mut pthread_once_t` raw-pointer
    // accessors.
    //
    // SAFETY: `once_control` was checked non-null above and is
    // assumed to point to a valid `pthread_once_t` per POSIX
    // contract.
    if unsafe { pthread_once_t::is_initialized_raw(once_control) }
        != pthread_once_t::IS_INITIALIZED_VALUE
    {
        ::syslog::warn!("pthread_once(): once_control not initialized with PTHREAD_ONCE_INIT");
        return ErrorCode::InvalidArgument.get();
    }

    // Fast path: already done.
    //
    // # Memory ordering
    //
    // POSIX requires that on return from `pthread_once`, the
    // effects of `init_routine` are visible.  On the fast path
    // this is provided by the volatile read + acquire compiler
    // fence: the read cannot be hoisted, and the fence prevents
    // subsequent loads from being reordered before the
    // observation of `ONCE_DONE`.
    //
    // On the current single-threaded target this fence is a
    // no-op at runtime (no SMP), but it is required for
    // correctness when the multi-threaded upgrade is performed.
    //
    // SAFETY: `once_control` is non-null and valid per the
    // caller's contract; see comment above on why a raw-pointer
    // accessor is used instead of `&mut`.
    let state_ptr: *mut c_int = unsafe { pthread_once_t::init_executed_ptr_raw(once_control) };
    {
        // `pthread_once_t` is `#[repr(C, packed)]`, so `state_ptr` may
        // be unaligned.  `ptr::read_volatile`/`write_volatile` require
        // alignment per Rust's spec (UB on unaligned access).  Use
        // `read_unaligned` + a compiler fence instead: the fence gives
        // the same compiler-reordering guarantee `volatile` provided
        // in this single-threaded model.  When the MT upgrade adds
        // SMP/futex coordination, this will need to become a proper
        // atomic load on an aligned word -- by then the struct's
        // alignment will have to be revisited too.
        let current = unsafe { ::core::ptr::read_unaligned(state_ptr) };
        ::core::sync::atomic::compiler_fence(::core::sync::atomic::Ordering::Acquire);
        if current == ONCE_DONE {
            return 0;
        }
        if current == ONCE_IN_PROGRESS {
            // POSIX (APPLICATION USAGE) leaves the behaviour of
            // recursive calls on the same `once_control` from
            // inside `init_routine` *undefined* -- it neither
            // mandates nor forbids returning.  In a
            // single-threaded model the only way to reach this
            // state is a recursive call: we choose to log and
            // return success without re-running, on the grounds
            // that (a) deadlocking is strictly worse and (b)
            // re-running could trigger unbounded recursion.  The
            // in-flight init will finish when control unwinds.
            ::syslog::warn!(
                "pthread_once(): recursive call on the same once_control (in-progress) -- not \
                 re-running init"
            );
            return 0;
        }
    }

    // Slow path: transition NEVER_RUN -> IN_PROGRESS, run init,
    // transition IN_PROGRESS -> DONE.
    //
    // # Multi-threaded upgrade
    //
    // Replace the volatile write with a CAS:
    //
    //     loop {
    //         match cas(state_ptr, NEVER_RUN, IN_PROGRESS) {
    //             Ok(_)              => break,            // we are the initializer
    //             Err(DONE)          => return 0,         // someone else finished
    //             Err(IN_PROGRESS)
    //               | Err(WAIT)      => {
    //                 cas(state_ptr, IN_PROGRESS, WAIT);
    //                 futex_wait(state_ptr, WAIT);
    //                 continue;
    //             }
    //             _                  => unreachable!(),
    //         }
    //     }
    //
    // and replace the final `write_volatile(DONE)` with an
    // atomic release-store followed by `futex_wake_all`.
    //
    // Use `write_unaligned` instead of `write_volatile` because
    // `pthread_once_t` is `#[repr(C, packed)]` and `state_ptr` may be
    // unaligned (see comment on the fast-path read above).  The
    // surrounding compiler fences provide the ordering guarantees
    // `volatile` would have given in ST mode.
    unsafe { ::core::ptr::write_unaligned(state_ptr, ONCE_IN_PROGRESS) };

    // POSIX cancellation semantics (reference, not active today):
    //
    //   "If init_routine is a cancellation point and is canceled,
    //    the effect on once_control shall be as if pthread_once()
    //    was never called."
    //
    // The Drop guard below is forward-compatibility scaffolding
    // for that contract.  In the current build it is effectively
    // dead code on the panic / cancellation path because:
    //
    //   1. `init_fn` is `extern "C"` (not `extern "C-unwind"`),
    //      so any panic that crosses the boundary is UB rather
    //      than a clean unwind.
    //   2. Release builds compile with `panic = "abort"`, so
    //      `Drop` impls never run on panic in the first place.
    //   3. Nanvix has no `pthread_cancel` today.
    //
    // The guard is retained so that, once any of the above
    // changes (C-unwind ABI, panic = "unwind", or real
    // cancellation), the cancellation-as-if-never-called
    // contract is honoured automatically without revisiting this
    // function.  In the MT upgrade `OnceGuard::drop` would also
    // call `futex_wake_all` on the state pointer.
    struct OnceGuard {
        state_ptr: *mut c_int,
        completed: bool,
    }
    impl ::core::ops::Drop for OnceGuard {
        fn drop(&mut self) {
            if !self.completed {
                // SAFETY: `state_ptr` was validated by the
                // caller of `pthread_once` and remains live
                // for the duration of this call.  `write_unaligned`
                // because `pthread_once_t` is `#[repr(C, packed)]`.
                unsafe {
                    ::core::ptr::write_unaligned(self.state_ptr, ONCE_NEVER_RUN);
                }
            }
        }
    }
    let mut guard = OnceGuard {
        state_ptr,
        completed: false,
    };

    // SAFETY: `init_fn` is a non-null `extern "C" fn()` per the
    // POSIX contract; the caller is responsible for ensuring it
    // does not violate Rust's aliasing rules on shared state.
    unsafe { init_fn() };

    guard.completed = true;
    drop(guard);

    // Release fence: ensures all stores performed by `init_fn`
    // are globally visible before `ONCE_DONE` becomes observable
    // on other CPUs.  No-op at runtime in ST/single-CPU mode;
    // required for correctness once SMP/MT lands.  `write_unaligned`
    // because `pthread_once_t` is `#[repr(C, packed)]`.
    ::core::sync::atomic::compiler_fence(::core::sync::atomic::Ordering::Release);
    unsafe { ::core::ptr::write_unaligned(state_ptr, ONCE_DONE) };

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
        ::syslog::warn!("pthread_attr_setdetachstate(): invalid attribute pointer");
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
        ::syslog::warn!("pthread_attr_setguardsize(): invalid attribute pointer");
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
        ::syslog::warn!("pthread_attr_setschedparam(): invalid attribute pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // Check if `param` is not valid.
    if param.is_null() {
        ::syslog::warn!("pthread_attr_setschedparam(): invalid sched param pointer");
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
        ::syslog::warn!("pthread_attr_setstack(): invalid attribute pointer");
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
        ::syslog::warn!("pthread_attr_setstackaddr(): invalid attribute pointer");
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
        ::syslog::warn!("pthread_setcanceltype(): invalid old type pointer");
        return ErrorCode::InvalidArgument.get();
    }

    // TODO: implement this function.
    ::syslog::warn!("pthread_setcanceltype(): not supported, failing");
    ErrorCode::OperationNotSupported.get()
}
