// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]
#![deny(clippy::all)]
#![deny(clippy::as_conversions)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

//! # Filesystem I/O After `fork()` + `execv()` Regression Test (caller)
//!
//! Acceptance test for the fork+exec+vfsd hang: a process reached via `fork()` then `execv()` must
//! be able to perform filesystem I/O through vfsd.
//!
//! `execv()` gives the exec'd image a new main-thread identifier; vfsd serves reads/writes through
//! a kernel push/pull rendezvous keyed by the client's `(pid, tid)`. Today the exec'd image's first
//! vfsd request never rendezvous-matches, so it blocks forever — any "fork then exec a program that
//! touches the filesystem" workload (a subprocess, `python script.py` from a server, ...) hangs.
//!
//! This caller forks and the child `execv()`s `/target` (built from `fork-exec-vfsd-target` and
//! bundled into the test's ramfs). The target performs a vfsd read and exits. The parent waits for
//! the child and only then writes the `"ok"` sentinel. While the bug is present the child hangs in
//! its vfsd read, so the parent's `waitpid()` never returns and the test times out; once fixed, the
//! child exits cleanly and the test passes.

//==================================================================================================
// Modules
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::{
    ffi::c_int,
    sys_types::pid_t,
    sys_wait::{
        wexitstatus,
        wifexited,
    },
    unistd::STDOUT_FILENO,
};
use ::syscall::unistd::{
    bindings,
    do_execv,
    write,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Path of the execv() target in the mounted ramfs (mounted at the filesystem root).
const TARGET_PATH: &str = "/target";

/// Exit status reported by the child if execv() returned (i.e. failed).
const CHILD_EXECV_FAILED: c_int = 127;

//==================================================================================================
// Test
//==================================================================================================

/// Verifies that a fork()+execv()'d image can perform vfsd filesystem I/O.
fn test_fork_exec_vfsd_io() -> Result<(), Error> {
    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        // Child: replace the image with the target. On success this never returns; the target runs
        // in place, performs its vfsd read and exits.
        let _error: Error = do_execv(TARGET_PATH, &["target"], &[]);
        // Only reached if execv() itself failed.
        // SAFETY: the child holds no resources requiring cleanup; terminate immediately.
        unsafe { bindings::_exit::_exit(CHILD_EXECV_FAILED) };
    }
    assert!(ret > 0, "fork() failed (ret={})", ret);

    // Parent: wait for the exec'd child. While the bug is present the child is stuck in its first
    // vfsd read and this never returns (the run times out).
    let mut wstatus: c_int = 0;
    // SAFETY: `wstatus` is a valid `c_int`.
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(ret, &raw mut wstatus, 0) };
    assert!(reaped == ret, "waitpid() must reap the child (ret={}, child={})", reaped, ret);
    assert!(
        wifexited(wstatus) && wexitstatus(wstatus) == 0,
        "fork()+execv()'d child failed its vfsd read (status={:#x})",
        wstatus
    );

    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!("fork-exec-vfsd-test: starting fork()+execv()+vfsd regression test");

    test_fork_exec_vfsd_io()?;
    ::syslog::info!("fork-exec-vfsd-test: PASS - fork_exec_vfsd_io");

    // Magic string consumed by the CI harness to mark a successful run. Only the parent reaches
    // this point; the child terminates inside the target via _exit().
    let magic_string: &[u8] = b"ok";
    write(STDOUT_FILENO, magic_string)?;

    Ok(())
}
