// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::sync::atomic::{
    AtomicI32,
    AtomicUsize,
    Ordering,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        pm::{
            __kcall_gettid,
            __kcall_lock_mutex,
            __kcall_unlock_mutex,
        },
        sched::__kcall_sched_yield,
    },
    pm::MutexAddress,
};
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    pthread::{
        PTHREAD_CREATE_DETACHED,
        PTHREAD_CREATE_JOINABLE,
        PTHREAD_MUTEX_INITIALIZER,
    },
    sched::sched_param,
    sys_types::{
        c_size_t,
        pthread_attr_t,
        pthread_mutex_t,
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

    // Nanvix does not currently provide guard areas for user thread stacks.
    *guardsize = 0;

    0
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
/// `init_executed` doubles as a small state-machine word:
///
/// - `ONCE_NEVER_RUN`: the initializer has not started.
/// - `ONCE_IN_PROGRESS`: some thread is currently running the initializer.
/// - `ONCE_DONE`: the initializer has completed and its effects are visible.
///
/// All reads and writes of this word are serialized by [`once_guard_addr()`]'s
/// process-global mutex.  The initializer itself runs *outside* that lock so that
/// (a) `pthread_once()` calls on unrelated control words can still make progress, and
/// (b) a call may recurse onto the same control word from within `init_routine`
/// without dead-locking (the recursive call observes `ONCE_IN_PROGRESS` and that the
/// current thread owns the in-flight init -- see [`ONCE_OWNERS`]).
const ONCE_NEVER_RUN: c_int = 0;
const ONCE_DONE: c_int = 1;
const ONCE_IN_PROGRESS: c_int = 2;

/// Process-global mutex serializing every `pthread_once()` state-word transition.
///
/// Only its *address* is meaningful: the kernel lazily creates a mutex keyed by the address on
/// first lock and never reads or writes the backing storage.  The static is therefore immutable;
/// all access goes through the raw `__kcall_lock_mutex` / `__kcall_unlock_mutex` kernel calls using
/// the address returned by [`once_guard_addr()`].
static ONCE_GUARD_MUTEX: pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;

/// Maximum number of `pthread_once_t` control words that may be running their
/// initializer simultaneously, summed across all threads (each distinct control counts
/// once, including nested/recursive inits started by the same thread).  Exceeding this
/// is pathological; callers receive `EAGAIN` and may retry.
const ONCE_OWNERS_MAX: usize = ::config::kernel::MAX_THREADS;

/// One slot of the in-progress owner table.
///
/// `control == 0` marks an empty slot.  Otherwise `control` is the `pthread_once_t`
/// pointer (as `usize`) whose initializer is running and `owner` is the raw identifier
/// of the thread running it.  A `pthread_once_t` is never at address `0` (it is
/// null-checked), so `0` is a safe empty sentinel.
struct OnceOwnerSlot {
    control: AtomicUsize,
    owner: AtomicI32,
}

/// Table tracking which thread is currently running the initializer for each in-flight
/// control word, used to distinguish a recursive self-call (return without re-running)
/// from a concurrent call by a different thread (wait until `ONCE_DONE`).
///
/// The table is only ever accessed while the once-guard mutex is held, so `Relaxed`
/// ordering is sufficient: the kernel mutex provides the happens-before edges between
/// threads, and Nanvix is single-core.  The atomics exist solely to share this `static`
/// across threads without forming a `&mut` to a `static mut`.
static ONCE_OWNERS: [OnceOwnerSlot; ONCE_OWNERS_MAX] = [const {
    OnceOwnerSlot {
        control: AtomicUsize::new(0),
        owner: AtomicI32::new(0),
    }
}; ONCE_OWNERS_MAX];

/// Returns the address of the once-guard mutex as a [`MutexAddress`].
///
/// The kernel keys mutexes by address and creates them on demand, so the value and type
/// of the backing static are irrelevant -- only its (stable, unique) address matters.
fn once_guard_addr() -> MutexAddress {
    // SAFETY: taking the address of a static never forms a reference and is always valid.
    MutexAddress::from(::core::ptr::addr_of!(ONCE_GUARD_MUTEX) as usize)
}

/// Releases the once-guard mutex.
///
/// On failure this logs and returns the kernel error number so the caller can surface it instead
/// of silently leaving the guard locked, which would wedge every later `pthread_once()` call that
/// contends on the process-global guard.
fn once_guard_unlock(guard_addr: MutexAddress) -> Result<(), c_int> {
    __kcall_unlock_mutex(guard_addr).map_err(|error| {
        ::syslog::warn!("pthread_once(): failed to release once guard ({error:?})");
        error.code.get()
    })
}

/// Records that thread `tid` is running the initializer for `control`.
///
/// Returns `false` if the table is full.  Must be called with the once-guard held.
fn once_owner_insert(control: usize, tid: i32) -> bool {
    for slot in ONCE_OWNERS.iter() {
        if slot.control.load(Ordering::Relaxed) == 0 {
            slot.control.store(control, Ordering::Relaxed);
            slot.owner.store(tid, Ordering::Relaxed);
            return true;
        }
    }
    false
}

/// Clears the owner entry for `control`, if present.
///
/// Must be called with the once-guard held.
fn once_owner_remove(control: usize) {
    for slot in ONCE_OWNERS.iter() {
        if slot.control.load(Ordering::Relaxed) == control {
            slot.control.store(0, Ordering::Relaxed);
            slot.owner.store(0, Ordering::Relaxed);
            return;
        }
    }
}

/// Returns the identifier of the thread running the initializer for `control`, if any.
///
/// Must be called with the once-guard held.
fn once_owner_of(control: usize) -> Option<i32> {
    for slot in ONCE_OWNERS.iter() {
        if slot.control.load(Ordering::Relaxed) == control {
            return Some(slot.owner.load(Ordering::Relaxed));
        }
    }
    None
}

/// In-flight guard for the `init_routine` call.
///
/// While `init_routine` runs, the control word is `ONCE_IN_PROGRESS` and this guard owns the
/// pending transition.  The in-flight init can end in one of two ways:
///
/// - **Normal completion.**  `pthread_once()` calls [`OnceGuard::finish()`], which re-acquires the
///   once-guard and, in one critical section, publishes `ONCE_DONE` and releases ownership.  This
///   is the only fallible finalization step, and its result is propagated to the caller so a kernel
///   mutex failure surfaces as an error number instead of a bogus success.
/// - **Unwind / cancellation.**  If `init_routine` does not return normally, `finish()` never runs
///   and the guard is still *armed*; its [`Drop`] then rolls the control word back to
///   `ONCE_NEVER_RUN` and releases ownership, honoring the POSIX contract that a canceled
///   `init_routine` leaves the control "as if `pthread_once()` was never called".
///
/// `Drop` is a best-effort fallback only: it cannot return a value, so it logs kernel errors
/// instead of propagating them.  The unwind path is effectively dead code today (release builds
/// compile with `panic = "abort"`, `init_routine` is `extern "C"` so a panic across it is UB rather
/// than a clean unwind, and Nanvix has no `pthread_cancel`), but is retained so the contract is
/// honored automatically if any of those change.  Re-acquiring the guard in either path is safe
/// because the lock is *not* held while `init_routine` runs.
struct OnceGuard {
    guard_addr: MutexAddress,
    state_ptr: *mut c_int,
    control_key: usize,
    /// `true` while `init_routine` is in flight, meaning `drop` must roll the control word back to
    /// `ONCE_NEVER_RUN`.  [`OnceGuard::finish()`] clears it after publishing `ONCE_DONE` on the
    /// normal-completion path, turning the subsequent `drop` into a no-op.
    armed: bool,
}

impl OnceGuard {
    /// Publishes `ONCE_DONE` and releases ownership on the normal-completion path.
    ///
    /// In one critical section, writes `ONCE_DONE` and clears the owner entry, then disarms the
    /// guard so the `drop` that runs as `self` falls out of scope does not roll the completed init
    /// back.  The guard is disarmed only *after* `ONCE_DONE` has been published and ownership has
    /// been released: if re-acquiring the once-guard fails, `self` stays armed so the `drop`
    /// rollback remains available instead of leaving the control word stuck at `ONCE_IN_PROGRESS`
    /// with a leaked owner-table entry.  Any kernel mutex error is returned so `pthread_once()` can
    /// surface it to the caller instead of reporting a bogus success.
    fn finish(mut self) -> Result<(), c_int> {
        // Re-acquire the guard *before* disarming.  If this fails, `self` is still armed, so the
        // `drop` that runs as it unwinds attempts the rollback rather than silently leaving the
        // control word stuck at `ONCE_IN_PROGRESS` with a leaked owner-table entry.
        __kcall_lock_mutex(self.guard_addr, None).map_err(|error| {
            ::syslog::warn!(
                "pthread_once(): failed to acquire once guard while publishing ONCE_DONE \
                 ({error:?})"
            );
            error.code.get()
        })?;
        // SAFETY: the guard is held, granting exclusive access to the state word.
        // `write_unaligned` because `pthread_once_t` is `#[repr(C, packed)]`.
        unsafe { ::core::ptr::write_unaligned(self.state_ptr, ONCE_DONE) };
        once_owner_remove(self.control_key);
        // Only now disarm: `ONCE_DONE` is published and ownership released, so the `drop` that runs
        // when `self` falls out of scope must not roll the completed init back.
        self.armed = false;
        once_guard_unlock(self.guard_addr)
    }
}

impl ::core::ops::Drop for OnceGuard {
    fn drop(&mut self) {
        // Best-effort fallback for the unwind/cancellation path only.  If `finish()` already ran
        // (normal completion) the guard is disarmed and there is nothing to do.  Otherwise
        // `init_routine` did not return normally, so roll the control word back to `ONCE_NEVER_RUN`
        // and release ownership, leaving the control "as if `pthread_once()` was never called".
        if !self.armed {
            return;
        }

        if let Err(error) = __kcall_lock_mutex(self.guard_addr, None) {
            ::syslog::warn!(
                "pthread_once(): failed to acquire once guard while rolling back init ({error:?})"
            );
            return;
        }
        // SAFETY: the guard is held, granting exclusive access to the state word.
        // `write_unaligned` because `pthread_once_t` is `#[repr(C, packed)]`.
        unsafe {
            ::core::ptr::write_unaligned(self.state_ptr, ONCE_NEVER_RUN);
        }
        once_owner_remove(self.control_key);
        if let Err(error) = __kcall_unlock_mutex(self.guard_addr) {
            ::syslog::warn!(
                "pthread_once(): failed to release once guard while rolling back init ({error:?})"
            );
        }
    }
}

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

    // `once_control` must have been initialized with `PTHREAD_ONCE_INIT`.  We deliberately do
    // NOT materialize a `&mut pthread_once_t`: a recursive call on the same control word from
    // inside `init_routine` is supported, and two live `&mut` to one object is UB.  All field
    // access goes through raw-pointer accessors.
    //
    // SAFETY: `once_control` was checked non-null above and is assumed to point to a valid
    // `pthread_once_t` per the POSIX contract.
    if unsafe { pthread_once_t::is_initialized_raw(once_control) }
        != pthread_once_t::IS_INITIALIZED_VALUE
    {
        ::syslog::warn!("pthread_once(): once_control not initialized with PTHREAD_ONCE_INIT");
        return ErrorCode::InvalidArgument.get();
    }

    // Identify the calling thread; used to distinguish a recursive self-call from a concurrent
    // call by a different thread.
    let tid: i32 = match __kcall_gettid() {
        Ok(tid) => i32::from(tid),
        Err(error) => {
            ::syslog::warn!("pthread_once(): gettid() failed ({error:?})");
            return error.code.get();
        },
    };

    let control_key: usize = once_control as usize;
    // SAFETY: `once_control` is valid per the contract above; the accessor never forms a
    // reference to the (`#[repr(C, packed)]`) object.
    let state_ptr: *mut c_int = unsafe { pthread_once_t::init_executed_ptr_raw(once_control) };
    let guard_addr: MutexAddress = once_guard_addr();

    loop {
        // Serialize all state-word transitions behind the process-global guard.
        if let Err(error) = __kcall_lock_mutex(guard_addr, None) {
            ::syslog::warn!("pthread_once(): failed to acquire once guard ({error:?})");
            return error.code.get();
        }

        // SAFETY: the guard is held, granting exclusive access to the state word.
        // `read_unaligned` because `pthread_once_t` is `#[repr(C, packed)]`.
        let state: c_int = unsafe { ::core::ptr::read_unaligned(state_ptr) };

        // Fast path: initialization already completed.  Holding the guard across this read
        // provides the happens-before edge that makes `init_routine`'s effects visible to this
        // thread on return, as POSIX requires.
        if state == ONCE_DONE {
            if let Err(code) = once_guard_unlock(guard_addr) {
                return code;
            }
            return 0;
        }

        // A thread is currently running the initializer.
        if state == ONCE_IN_PROGRESS {
            let owner: Option<i32> = once_owner_of(control_key);
            if let Err(code) = once_guard_unlock(guard_addr) {
                return code;
            }

            // Recursive call on the same control from within `init_routine`.  POSIX leaves this
            // undefined; we return success without re-running rather than dead-lock or recurse
            // without bound.  The in-flight init finishes when control unwinds.
            if owner == Some(tid) {
                ::syslog::warn!(
                    "pthread_once(): recursive call on the same once_control (in-progress) -- not \
                     re-running init"
                );
                return 0;
            }

            // A different thread owns the in-flight init.  Yield and retry until it reaches
            // `ONCE_DONE`; the initializer runs outside the guard, so it makes progress.
            if let Err(error) = __kcall_sched_yield() {
                ::syslog::warn!("pthread_once(): sched_yield() failed ({error:?})");
                return error.code.get();
            }
            continue;
        }

        // If the state word contains an unexpected value, treat it as an invalid control rather
        // than silently re-running the initializer.
        if state != ONCE_NEVER_RUN {
            if let Err(code) = once_guard_unlock(guard_addr) {
                return code;
            }
            ::syslog::warn!("pthread_once(): invalid once_control state ({state})");
            return ErrorCode::InvalidArgument.get();
        }

        // `ONCE_NEVER_RUN`: become the initializer. Record ownership before publishing
        // `ONCE_IN_PROGRESS` so a recursive self-call is always recognized.
        if !once_owner_insert(control_key, tid) {
            if let Err(code) = once_guard_unlock(guard_addr) {
                return code;
            }
            ::syslog::warn!("pthread_once(): in-progress owner table full -- retry later");
            return ErrorCode::TryAgain.get();
        }
        // SAFETY: guard held; `write_unaligned` because the struct is packed.
        unsafe { ::core::ptr::write_unaligned(state_ptr, ONCE_IN_PROGRESS) };
        if let Err(code) = once_guard_unlock(guard_addr) {
            return code;
        }

        // Run the initializer OUTSIDE the guard so that (a) `pthread_once()` on unrelated
        // controls can proceed and (b) a recursive self-call can acquire the guard, observe our
        // ownership, and return.  While `init_routine` is in flight the guard is armed, so it
        // reverts the state word if `init_routine` unwinds.
        let guard = OnceGuard {
            guard_addr,
            state_ptr,
            control_key,
            armed: true,
        };
        // SAFETY: `init_fn` is a valid `extern "C" fn()` per the POSIX contract; the caller is
        // responsible for ensuring it does not violate Rust's aliasing rules on shared state.
        unsafe { init_fn() };

        // Normal completion: publish `ONCE_DONE` and release ownership explicitly so a kernel mutex
        // failure is propagated to the caller instead of being swallowed by `drop` (which cannot
        // return a value).  `finish()` disarms the guard, so its `drop` rollback runs only if
        // `init_routine` unwound before reaching this point.
        return match guard.finish() {
            Ok(()) => 0,
            Err(code) => code,
        };
    }
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

    // Check if `detachstate` is not valid.
    if detachstate != PTHREAD_CREATE_JOINABLE && detachstate != PTHREAD_CREATE_DETACHED {
        ::syslog::warn!("pthread_attr_setdetachstate(): invalid detach state");
        return ErrorCode::InvalidArgument.get();
    }

    // Store the detach state.
    (*attr).detachstate = detachstate;

    0
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

    // Store the scheduling parameters.
    (*attr).schedparam = *param;

    0
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
