// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::KernelThread;
use ::config::memory_layout::USER_THREAD_STACK_SIZE;
use ::core::ptr;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::sched::__kcall_sched_yield,
};
use ::sysapi::sys_types::pthread_t;
use ::syscall::pthread::{
    Pointer,
    pthread_create,
    pthread_getspecific,
    pthread_join,
    pthread_key_create,
    pthread_key_delete,
    pthread_setspecific,
};

//==================================================================================================
// Constants
//==================================================================================================

const EXPECTED_EXIT_STATUS: usize = 0xdeadbeef;
const TLS_INITIAL_VALUE: usize = 0x1234;
const TLS_MAIN_VALUE: usize = 0x5678;
const TLS_WORKER_VALUES: [usize; 2] = [0xabcd, 0xcdef];
const TLS_YIELDS: usize = 8;

//==================================================================================================
// Structures
//==================================================================================================

#[repr(align(64))]
struct TlsValue(usize);

//==================================================================================================
// Globals
//==================================================================================================

#[thread_local]
static mut TLS_INITIALIZED: TlsValue = TlsValue(TLS_INITIAL_VALUE);

#[thread_local]
static mut TLS_ZEROED: TlsValue = TlsValue(0);

/// Sentinel values whose *addresses* are stored as thread-specific data.
static mut MAIN_DATA: usize = 0xfeedface;
static mut WORKER_DATA: usize = 0xcafebabe;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests compiler TLS and thread-specific data via the pthread key APIs.
pub fn run() -> Result<(), Error> {
    test_compiler_thread_local_storage()?;
    test_thread_specific_data()?;
    Ok(())
}

fn test_compiler_thread_local_storage() -> Result<(), Error> {
    assert_eq!(tls_values(), (TLS_INITIAL_VALUE, 0), "main TLS initialization mismatch");
    set_tls_values(TLS_MAIN_VALUE);

    let workers: [pthread_t; 2] = [
        pthread_create(tls_worker_entry, TLS_WORKER_VALUES[0], USER_THREAD_STACK_SIZE)?,
        pthread_create(tls_worker_entry, TLS_WORKER_VALUES[1], USER_THREAD_STACK_SIZE)?,
    ];

    for _ in 0..TLS_YIELDS {
        __kcall_sched_yield()?;
        assert_eq!(tls_values(), (TLS_MAIN_VALUE, TLS_MAIN_VALUE + 1), "main TLS corrupted");
    }
    for (worker, expected) in workers.into_iter().zip(TLS_WORKER_VALUES) {
        assert_eq!(pthread_join(worker)?, expected, "worker returned unexpected TLS value");
    }

    // A new thread must start from the template, not from a previous thread's modified TLS.
    let worker: pthread_t =
        pthread_create(tls_worker_entry, TLS_WORKER_VALUES[0], USER_THREAD_STACK_SIZE)?;
    assert_eq!(pthread_join(worker)?, TLS_WORKER_VALUES[0], "replacement worker TLS mismatch");
    assert_eq!(tls_values(), (TLS_MAIN_VALUE, TLS_MAIN_VALUE + 1), "main TLS corrupted");
    Ok(())
}

extern "C" fn tls_worker_entry(value: usize) -> usize {
    tls_worker_impl(value).unwrap_or_else(|err| panic!("tls_worker: {err:?}"))
}

fn tls_worker_impl(value: usize) -> Result<usize, Error> {
    assert_eq!(tls_values(), (TLS_INITIAL_VALUE, 0), "worker TLS initialization mismatch");
    set_tls_values(value);
    for _ in 0..TLS_YIELDS {
        __kcall_sched_yield()?;
        assert_eq!(tls_values(), (value, value + 1), "worker TLS corrupted");
    }
    Ok(tls_values().0)
}

#[inline(never)]
fn tls_values() -> (usize, usize) {
    // SAFETY: These pointers refer only to the calling thread's initialized TLS. Volatile accesses
    // ensure the test exercises compiler-generated TLS addressing instead of constant propagation.
    unsafe {
        let initialized: *const usize = ptr::addr_of!(TLS_INITIALIZED.0);
        let zeroed: *const usize = ptr::addr_of!(TLS_ZEROED.0);
        assert!(
            initialized
                .addr()
                .is_multiple_of(core::mem::align_of::<TlsValue>())
        );
        assert!(
            zeroed
                .addr()
                .is_multiple_of(core::mem::align_of::<TlsValue>())
        );
        (initialized.read_volatile(), zeroed.read_volatile())
    }
}

#[inline(never)]
fn set_tls_values(value: usize) {
    // SAFETY: Each thread writes only its own TLS, without creating references to mutable statics.
    unsafe {
        ptr::addr_of_mut!(TLS_INITIALIZED.0).write_volatile(value);
        ptr::addr_of_mut!(TLS_ZEROED.0).write_volatile(value + 1);
    }
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
