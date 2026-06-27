// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    convert::TryFrom,
    ptr,
};
use ::sys::{
    kcall::pm::__kcall_capctl,
    pm::{
        Capability,
        MutexAddress,
        ThreadCreateArgs,
    },
};
use ::sysapi::{
    pthread::PTHREAD_MUTEX_INITIALIZER,
    sys_types::pthread_mutex_t,
};
use ::syscall::safe::mem::stack::Stack;

//==================================================================================================
// Globals
//==================================================================================================

static mut STRESS_MUTEX: pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// RAII guard that enables a capability upon creation and disables it when dropped.
///
/// # Parameters
///
/// - `capability`: Capability toggled for the lifetime of the guard.
///
/// # Errors
///
/// Propagates capability control errors from the underlying kernel call.
///
pub struct CapabilityGuard {
    capability: Capability,
    released: bool,
}

impl CapabilityGuard {
    ///
    /// # Description
    ///
    /// Enables the provided capability and returns a guard that will disable it on drop.
    ///
    /// # Parameters
    ///
    /// - `capability`: Capability to enable.
    ///
    /// # Returns
    ///
    /// Guard managing the capability lifetime.
    ///
    /// # Errors
    ///
    /// Fails if the capability cannot be enabled.
    ///
    pub fn enable(capability: Capability) -> Result<Self, StressError> {
        __kcall_capctl(capability, true)?;
        Ok(Self {
            capability,
            released: false,
        })
    }

    ///
    /// # Description
    ///
    /// Explicitly disables the capability if it is still active.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the capability is disabled or already released.
    ///
    /// # Errors
    ///
    /// Propagates errors from the capability control call.
    ///
    pub fn disable(&mut self) -> Result<(), StressError> {
        if !self.released {
            __kcall_capctl(self.capability, false)?;
            self.released = true;
        }
        Ok(())
    }
}

impl Drop for CapabilityGuard {
    fn drop(&mut self) {
        if !self.released {
            let _ = __kcall_capctl(self.capability, false);
            self.released = true;
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the mutex address shared by stress workloads that need mutual exclusion.
///
/// # Safety
///
/// Uses a process-wide static mutex; callers must pair with `reset_stress_mutex` when needed.
///
pub fn stress_mutex_addr() -> MutexAddress {
    let ptr: *mut pthread_mutex_t = ptr::addr_of_mut!(STRESS_MUTEX);
    MutexAddress::from(raw_pointer_address(ptr))
}

///
/// # Description
///
/// Reinitializes the shared stress mutex back to `PTHREAD_MUTEX_INITIALIZER`.
///
/// # Safety
///
/// Safe as long as no other thread holds the mutex when called.
///
pub fn reset_stress_mutex() {
    unsafe {
        STRESS_MUTEX = PTHREAD_MUTEX_INITIALIZER;
    }
}

///
/// # Description
///
/// Builds thread creation arguments for a stress worker with an explicit entry point and stack.
///
/// # Parameters
///
/// - `stack`: User stack allocated for the worker.
/// - `entry`: Entry function pointer executed by the worker.
/// - `arg`: First argument passed to the worker.
///
/// # Returns
///
/// Thread creation arguments ready for `create_thread`.
///
pub fn thread_args(
    stack: &Stack,
    entry: extern "C" fn(usize) -> usize,
    arg: usize,
) -> ThreadCreateArgs {
    ThreadCreateArgs {
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: raw_entry_address(entry),
        user_fn_arg1: arg,
        user_stack_base: stack.base(),
        user_stack_size: stack.size(),
        user_tda: None,
    }
}

///
/// # Description
///
/// Converts a worker entry function pointer into its raw address representation.
///
/// # Parameters
///
/// - `entry`: Worker entry point used for thread creation.
///
/// # Returns
///
/// Raw address of the entry function.
#[allow(clippy::as_conversions, clippy::fn_to_numeric_cast)]
fn raw_entry_address(entry: extern "C" fn(usize) -> usize) -> usize {
    entry as usize
}

///
/// # Description
///
/// Converts a mutable pointer into its raw address representation.
///
/// # Parameters
///
/// - `ptr`: Pointer to convert.
///
/// # Returns
///
/// Raw address of the pointer.
#[allow(clippy::as_conversions)]
pub fn raw_pointer_address<T>(ptr: *mut T) -> usize {
    ptr as usize
}

///
/// # Description
///
/// Interprets a raw address as a mutable `u8` pointer.
///
/// # Parameters
///
/// - `raw_addr`: Raw address to reinterpret.
///
/// # Returns
///
/// Mutable pointer to the supplied address.
#[allow(clippy::as_conversions)]
pub fn exposed_addr_to_mut_u8(raw_addr: usize) -> *mut u8 {
    raw_addr as *mut u8
}

/// Exports Stack so test modules can allocate per-thread stacks with the expected type alias.
pub use ::syscall::safe::mem::stack::Stack as WorkerStack;

/// Re-exports Error so modules can shorten their signatures.
pub use ::sys::error::Error as StressError;

///
/// # Description
///
/// 32-bit xorshift PRNG for deterministic pseudo-random sequences in stress tests.
///
/// # Parameters
///
/// - `state`: Current PRNG state (must be non-zero for useful output).
///
/// # Returns
///
/// Next PRNG state.
///
pub fn xorshift32(state: u32) -> u32 {
    let mut s: u32 = state;
    s ^= s << 13;
    s ^= s >> 17;
    s ^= s << 5;
    s
}

///
/// # Description
///
/// Converts an `ErrorCode` to `usize` without relying on unsafe `as` casts.
///
/// # Returns
///
/// Numeric representation of the error code suitable for atomics or raw storage.
pub fn error_code_to_usize(code: ::sys::error::ErrorCode) -> usize {
    match usize::try_from(u32::from(code)) {
        Ok(value) => value,
        Err(_) => usize::MAX,
    }
}

///
/// # Description
///
/// Attempts to reconstruct an `ErrorCode` from a raw `usize` captured earlier.
/// Falls back to `InvalidArgument` if conversion fails or the value is unknown.
///
/// # Parameters
///
/// - `raw`: Raw numeric value that encodes an `ErrorCode`.
///
/// # Returns
///
/// Parsed `ErrorCode` or `InvalidArgument` on failure.
pub fn error_code_from_usize(raw: usize) -> ::sys::error::ErrorCode {
    match i64::try_from(raw)
        .ok()
        .and_then(|value| ::sys::error::ErrorCode::try_from(value).ok())
    {
        Some(code) => code,
        None => ::sys::error::ErrorCode::InvalidArgument,
    }
}
