// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! This Rust program tests if mutexes can be used to synchronize access to global variables.  It
//! creates a worker thread that writes a magic string to the standard output and then exits.  The
//! main thread waits for the worker thread to signal that it is initialized and then waits for the
//! worker thread to exit.
//!

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::{
    mm::VirtualAddress,
    pm::ThreadIdentifier,
    sys::{
        error::Error,
        kcall::{
            pm,
            sched,
        },
    },
};
use ::posix::{
    sys::types::size_t,
    unistd,
};

//==================================================================================================
// Mutex Structure
//==================================================================================================

struct Mutex {
    addr: usize,
}

impl Mutex {
    pub const fn new() -> Self {
        Self { addr: 0 }
    }

    /// Locks the mutex.
    fn lock(&self) -> Result<(), Error> {
        pm::lock_mutex(VirtualAddress::from_raw_value(&self.addr as *const usize as usize))
    }

    /// Unlocks the mutex.
    fn unlock(&self) -> Result<(), Error> {
        pm::unlock_mutex(VirtualAddress::from_raw_value(&self.addr as *const usize as usize))
    }
}

unsafe impl Sync for Mutex {}
unsafe impl Send for Mutex {}

//==================================================================================================
// Constants
//==================================================================================================

/// Expected identifiers of the master thread.
const EXPECTED_MASTER_TID: usize = 1;

/// Expected identifiers of the worker thread.
const EXPECTED_WORKER_TID: usize = 2;

/// Expected argument passed to the worker thread.
const EXPECTED_WORKER_ARG: usize = 0xbadcafe;

/// Expected exit status of the worker thread.
const EXPECTED_EXIT_STATUS: usize = 0xdeadbeef;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Global mutex used to synchronize access to global variables.
static MUTEX: Mutex = Mutex::new();

/// Global variable used to signal that the worker thread is initialized.
static mut INITIALIZED: bool = false;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests if mutexes can be used to synchronize access to global variables.
#[no_mangle]
pub fn main() -> Result<(), Error> {
    // Get the master thread identifier and check if it matches the expected value.
    let master_tid: ThreadIdentifier = pm::gettid().unwrap();
    assert_eq!(master_tid, ThreadIdentifier::from(EXPECTED_MASTER_TID));

    // Create a worker thread and check if its identifier matches the expected value.
    let worker_tid: ThreadIdentifier = pm::create_thread(worker, EXPECTED_WORKER_ARG).unwrap();
    assert_eq!(worker_tid, ThreadIdentifier::from(EXPECTED_WORKER_TID));

    // Wait for the worker thread to complete.
    loop {
        // Obtain a cached copy of the initialized flag.
        MUTEX.lock().unwrap();
        let completed: bool = unsafe { INITIALIZED };
        MUTEX.unlock().unwrap();

        if completed {
            break;
        }

        sched::sched_yield().unwrap();
    }

    // Wait for the worker thread to exit and check if it returns the expected value.
    let mut retval: usize = 0;
    pm::join_thread(worker_tid, &mut retval).unwrap();
    assert_eq!(retval, EXPECTED_EXIT_STATUS);

    // Write magic string to signal that the test passed.
    {
        let magic_string: &[u8] = "ok".as_bytes();
        unistd::write(unistd::STDOUT_FILENO, magic_string.as_ptr(), magic_string.len() as size_t);
    }

    Ok(())
}

/// Worker thread.
extern "C" fn worker(arg: usize) -> usize {
    // Check if worker argument matches the expected value.
    assert_eq!(arg, EXPECTED_WORKER_ARG);

    // Get the worker thread identifier and check if it matches the expected value.
    let worker_tid: ThreadIdentifier = pm::gettid().unwrap();
    assert_eq!(worker_tid, ThreadIdentifier::from(EXPECTED_WORKER_TID));

    // Signal that the worker thread is initialized.
    MUTEX.lock().unwrap();
    unsafe {
        INITIALIZED = true;
    }
    MUTEX.unlock().unwrap();

    // Exit the worker thread and make sure it returns the expected value.
    let error = pm::exit_thread(EXPECTED_EXIT_STATUS).unwrap_err();
    unreachable!("failed to exit thread: {:?}", error);
}
