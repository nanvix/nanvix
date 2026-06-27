// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::KernelThread;
use ::core::ptr;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::syscall::pthread::{
    Pointer,
    pthread_getspecific,
    pthread_key_create,
    pthread_key_delete,
    pthread_setspecific,
};

// TODO: Port thread_local.c (_Thread_local / C11 thread-local storage). The C test uses the
// `_Thread_local` language keyword which has no direct kernel-call equivalent in Rust `no_std`.
// A future port would require compiler-level TLS support for the Nanvix target.

//==================================================================================================
// Constants
//==================================================================================================

const EXPECTED_EXIT_STATUS: usize = 0xdeadbeef;

//==================================================================================================
// Globals
//==================================================================================================

/// Sentinel values whose *addresses* are stored as thread-specific data.
static mut MAIN_DATA: usize = 0xfeedface;
static mut WORKER_DATA: usize = 0xcafebabe;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests thread-specific data via `pthread_key_create` / `pthread_setspecific` /
/// `pthread_getspecific` (ports tda.c).
pub fn run() -> Result<(), Error> {
    test_thread_specific_data()?;
    Ok(())
}

fn test_thread_specific_data() -> Result<(), Error> {
    // Create a key for thread-specific data.
    let key = pthread_key_create()
        .ok_or_else(|| Error::new(ErrorCode::OutOfMemory, "pthread_key_create failed"))?;

    // Set thread-specific data in the main thread.
    let main_ptr: *mut usize = ptr::addr_of_mut!(MAIN_DATA);
    pthread_setspecific(key, Pointer::from(main_ptr))?;

    // Verify main thread can retrieve its value.
    let retrieved: Pointer = pthread_getspecific(key)?;
    let retrieved_ptr: *mut usize = retrieved.into();
    assert_eq!(retrieved_ptr, main_ptr, "main thread TSD mismatch");

    // Spawn worker that sets its own value for the same key.
    let thread = KernelThread::spawn(tsd_worker_entry, key_to_arg(key))?;
    let retval = thread.join()?;
    assert_eq!(retval, EXPECTED_EXIT_STATUS, "worker returned unexpected status");

    // Main thread's value must still be intact after worker ran.
    let after: Pointer = pthread_getspecific(key)?;
    let after_ptr: *mut usize = after.into();
    assert_eq!(after_ptr, main_ptr, "main thread TSD corrupted by worker");

    // Clean up.
    pthread_key_delete(key)?;

    Ok(())
}

extern "C" fn tsd_worker_entry(arg: usize) -> usize {
    tsd_worker_impl(arg).unwrap_or_else(|err| panic!("tsd_worker: {err:?}"))
}

fn tsd_worker_impl(arg: usize) -> Result<usize, Error> {
    let key = arg_to_key(arg);

    // Set worker-specific value.
    let worker_ptr: *mut usize = ptr::addr_of_mut!(WORKER_DATA);
    pthread_setspecific(key, Pointer::from(worker_ptr))?;

    // Retrieve and verify.
    let retrieved: Pointer = pthread_getspecific(key)?;
    let retrieved_ptr: *mut usize = retrieved.into();
    assert_eq!(retrieved_ptr, worker_ptr, "worker thread TSD mismatch");

    Ok(EXPECTED_EXIT_STATUS)
}

//==================================================================================================
// Helpers
//==================================================================================================

/// Converts a `pthread_key_t` (u32) into a `usize` argument for thread entry.
fn key_to_arg(key: ::sysapi::sys_types::pthread_key_t) -> usize {
    #[allow(clippy::as_conversions)]
    let arg: usize = key as usize;
    arg
}

/// Converts a `usize` argument back into a `pthread_key_t`.
fn arg_to_key(arg: usize) -> ::sysapi::sys_types::pthread_key_t {
    // pthread_key_t is u32; truncation is safe because key values are small indices.
    #[allow(clippy::as_conversions)]
    let key = arg as ::sysapi::sys_types::pthread_key_t;
    key
}
