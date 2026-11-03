// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use crate::signal::sigset_t;

#[test]
fn test_sigemptyset() {
    let mut set: sigset_t = u64::MAX;
    assert_eq!(unsafe { sigemptyset(&mut set) }, 0);
    assert_eq!(set, 0);
}

#[test]
fn test_sigfillset() {
    let mut set: sigset_t = 0;
    assert_eq!(unsafe { sigfillset(&mut set) }, 0);
    assert_eq!(set, u64::MAX);
}

#[test]
fn test_sigisemptyset() {
    let mut set: sigset_t = 0;
    assert_eq!(unsafe { sigemptyset(&mut set) }, 0);
    assert_eq!(unsafe { sigisemptyset(&set) }, 1);

    // A set with any signal is not empty.
    assert_eq!(unsafe { sigaddset(&mut set, 1) }, 0);
    assert_eq!(unsafe { sigisemptyset(&set) }, 0);

    // A full set is not empty.
    assert_eq!(unsafe { sigfillset(&mut set) }, 0);
    assert_eq!(unsafe { sigisemptyset(&set) }, 0);
}

#[test]
fn test_sigaddset_and_sigismember() {
    let mut set: sigset_t = 0;
    assert_eq!(unsafe { sigemptyset(&mut set) }, 0);

    // Add signal 1.
    assert_eq!(unsafe { sigaddset(&mut set, 1) }, 0);
    assert_eq!(unsafe { sigismember(&set, 1) }, 1);
    assert_eq!(unsafe { sigismember(&set, 2) }, 0);

    // Add signal 15.
    assert_eq!(unsafe { sigaddset(&mut set, 15) }, 0);
    assert_eq!(unsafe { sigismember(&set, 15) }, 1);
    assert_eq!(unsafe { sigismember(&set, 1) }, 1);
}

#[test]
fn test_sigdelset() {
    let mut set: sigset_t = 0;
    assert_eq!(unsafe { sigfillset(&mut set) }, 0);

    // Remove signal 9.
    assert_eq!(unsafe { sigdelset(&mut set, 9) }, 0);
    assert_eq!(unsafe { sigismember(&set, 9) }, 0);
    assert_eq!(unsafe { sigismember(&set, 10) }, 1);
}

#[test]
fn test_sigset_invalid_signum() {
    let mut set: sigset_t = 0;
    assert_eq!(unsafe { sigaddset(&mut set, 0) }, -1);
    assert_eq!(unsafe { sigaddset(&mut set, 65) }, -1);
    assert_eq!(unsafe { sigdelset(&mut set, -1) }, -1);
    assert_eq!(unsafe { sigismember(&set, 0) }, -1);
}

#[test]
fn test_sigset_null_pointer() {
    assert_eq!(unsafe { sigemptyset(core::ptr::null_mut()) }, -1);
    assert_eq!(unsafe { sigfillset(core::ptr::null_mut()) }, -1);
    assert_eq!(unsafe { sigisemptyset(core::ptr::null()) }, -1);
    assert_eq!(unsafe { sigaddset(core::ptr::null_mut(), 1) }, -1);
    assert_eq!(unsafe { sigdelset(core::ptr::null_mut(), 1) }, -1);
    assert_eq!(unsafe { sigismember(core::ptr::null(), 1) }, -1);
}

#[test]
fn test_sigset_boundary_signals() {
    let mut set: sigset_t = 0;
    assert_eq!(unsafe { sigemptyset(&mut set) }, 0);

    // Signal 1 (minimum valid).
    assert_eq!(unsafe { sigaddset(&mut set, 1) }, 0);
    assert_eq!(unsafe { sigismember(&set, 1) }, 1);

    // Signal 64 (maximum valid).
    assert_eq!(unsafe { sigaddset(&mut set, 64) }, 0);
    assert_eq!(unsafe { sigismember(&set, 64) }, 1);
}
