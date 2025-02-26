// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! This Rust program tests the creation and joining of threads in a no-std environment using the
//! Nanvix kernel interface. The program consists of a main function that creates a worker thread
//! and waits for it to exit, and a worker function that performs some operations and then exits.
//!

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::{
    pm::ThreadIdentifier,
    sys::{
        error::{
            Error,
            ErrorCode,
        },
        kcall::pm,
    },
};
use ::posix::{
    sys::types::size_t,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Expected identifiers of the master thread.
const EXPECTED_MASTER_TID: usize = 1;

/// Expected identifiers of the worker thread.
const EXPECTED_WORKER_TID: usize = 2;

/// Expected exit status of the worker thread.
const EXPECTED_EXIT_STATUS: usize = 0xdeadbeef;

/// Tests the creation and joining of threads using the Nanvix kernel interface.
#[no_mangle]
pub fn main() -> Result<(), Error> {
    // Get the master thread identifier and check if it matches the expected value.
    let master_tid: ThreadIdentifier = pm::gettid().unwrap();
    assert_eq!(master_tid, ThreadIdentifier::from(EXPECTED_MASTER_TID));

    // Create a worker thread and check if its identifier matches the expected value.
    let worker_tid: ThreadIdentifier = pm::create_thread(worker).unwrap();
    assert_eq!(worker_tid, ThreadIdentifier::from(EXPECTED_WORKER_TID));

    // Wait for the worker thread to exit and check if it returns the expected value.
    let mut retval: usize = 0;
    loop {
        match pm::join_thread(worker_tid, &mut retval) {
            Ok(_) => break,
            Err(error) if error.code != ErrorCode::OperationWouldBlock => {
                break;
            },
            _ => continue,
        }
    }
    assert_eq!(retval, EXPECTED_EXIT_STATUS);

    // Write magic string to signal that the test passed.
    {
        let magic_string: &[u8] = "ok".as_bytes();
        unistd::write(unistd::STDOUT_FILENO, magic_string.as_ptr(), magic_string.len() as size_t);
    }

    Ok(())
}

/// Worker thread.
fn worker() -> ! {
    // Get the worker thread identifier and check if it matches the expected value.
    let worker_tid: ThreadIdentifier = pm::gettid().unwrap();
    assert_eq!(worker_tid, ThreadIdentifier::from(EXPECTED_WORKER_TID));

    // Exit the worker thread and make sure it returns the expected value.
    let error = pm::exit_thread(EXPECTED_EXIT_STATUS).unwrap_err();
    unreachable!("failed to exit thread: {:?}", error);
}
