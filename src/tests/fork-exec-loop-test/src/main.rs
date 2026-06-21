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

//! # File-Descriptor Inheritance Across `fork()` + `execv()` Regression Test (caller)
//!
//! Acceptance test: a file descriptor opened by the parent before `fork()` must remain usable by the
//! child after it `execv()`s a new image -- POSIX requires open descriptors to survive `execv()`
//! unless they are marked `FD_CLOEXEC`. Each iteration the parent opens `/coldfile.dat`, forks, and
//! the child `execv()`s `/target` (`fork-exec-loop-target`) passing the descriptor number as
//! `argv[1]`; the target reads `/coldfile.dat`'s contents through that INHERITED descriptor.
//!
//! On Nanvix a freshly opened (non-`CLOEXEC`) descriptor is NOT usable by a `fork()`+`execv()`'d
//! child: although a plain `fork()` correctly shares the parent's descriptors (see
//! `test-fork-guestfs`), the descriptor table is not carried onto the child once it `execv()`s, so
//! the inherited descriptor is unknown in the exec'd image and the read fails. vfsd clones the
//! parent's per-process descriptor table onto the child asynchronously (in response to a
//! fire-and-forget notification from procd); the exec'd image races ahead of -- or otherwise does
//! not receive -- that clone, sometimes accompanied by the log line `failed to clone filesystem
//! state (... error=AlreadyExists)`.
//!
//! This is a concrete, deterministic facet of the broader fork+exec filesystem-state handling that
//! also makes a `fork()`+`execv()`'d CPython abort during interpreter initialization (e.g.
//! "init_import_site: Failed to import the site module") when a long-running parent (such as an HTTP
//! server) spawns interpreter subprocesses. The test loops [`ITERATIONS`] times so a partial fix
//! that only works occasionally still fails it.
//!
//! The caller requires EVERY child to exit 0. The first child that does not (including an `execv()`
//! failure surfaced as [`CHILD_EXECV_FAILED`]) fails the test. While the bug is present the test
//! FAILS; once a descriptor opened before `fork()` survives the child's `execv()`, it passes and
//! guards the behavior.
//!
//! `/coldfile.dat` and `/target` are bundled into the test ramfs by the harness.

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
    fcntl::file_access_mode::O_RDONLY,
    ffi::c_int,
    sys_types::{
        mode_t,
        pid_t,
    },
    sys_wait::{
        wexitstatus,
        wifexited,
    },
    unistd::STDOUT_FILENO,
};
use ::syscall::{
    fcntl::open,
    unistd::{
        bindings,
        close,
        do_execv,
        write,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Path of the execv() target in the mounted ramfs (mounted at the filesystem root).
const TARGET_PATH: &str = "/target";

/// File the parent opens before each fork; the child reads it through the inherited descriptor.
const COLD_PATH: &str = "/coldfile.dat";

/// Number of fork()+execv() cycles to perform. The bug reproduces on the first cycle; the loop
/// guards against a partial fix that only preserves the inherited descriptor some of the time.
const ITERATIONS: usize = 32;

/// Exit status reported by the child if execv() returned (i.e. failed).
const CHILD_EXECV_FAILED: c_int = 127;

//==================================================================================================
// Helpers
//==================================================================================================

/// Formats `value` as decimal into `buf` and returns a `&str` borrowing the digit bytes for use as
/// an argv entry. A trailing NUL is also written into `buf` so the buffer holds a C string, but it
/// is not part of the returned slice. `buf` must be large enough for the digits plus a trailing NUL
/// (12 bytes suffices for any non-negative `i32`).
fn format_fd(value: c_int, buf: &mut [u8; 12]) -> &str {
    let mut tmp: [u8; 11] = [0u8; 11];
    let mut i: usize = tmp.len();
    let mut v: u32 = u32::try_from(value).unwrap_or(0);
    loop {
        i -= 1;
        tmp[i] = b'0' + u8::try_from(v % 10).unwrap_or(0);
        v /= 10;
        if v == 0 {
            break;
        }
    }
    let digits: &[u8] = &tmp[i..];
    buf[..digits.len()].copy_from_slice(digits);
    buf[digits.len()] = 0;
    // The bytes are ASCII digits followed by a NUL, hence valid UTF-8.
    ::core::str::from_utf8(&buf[..digits.len()]).unwrap_or("0")
}

//==================================================================================================
// Test
//==================================================================================================

/// Verifies that the parent can fork()+execv() a child that reads an inherited descriptor,
/// repeatedly.
fn test_fork_exec_loop() -> Result<(), Error> {
    let mode: mode_t = 0;

    for i in 0..ITERATIONS {
        // Open the data file in the parent. The child must inherit this descriptor across
        // fork()+execv() and read the file through it.
        let fd: c_int = open(COLD_PATH, O_RDONLY, mode)?;

        let mut fd_buf: [u8; 12] = [0u8; 12];
        let fd_arg: &str = format_fd(fd, &mut fd_buf);

        let ret: pid_t = bindings::fork::fork();
        if ret == 0 {
            // Child: exec the target, handing it the inherited descriptor number.
            let _error: Error = do_execv(TARGET_PATH, &["target", fd_arg], &[]);
            // Only reached if execv() itself failed (e.g. the loader could not read the target).
            // SAFETY: the child holds no resources requiring cleanup; terminate immediately.
            unsafe { bindings::_exit::_exit(CHILD_EXECV_FAILED) };
        }
        if ret < 0 {
            // fork() failed: release the descriptor opened above before aborting so a failing
            // fork() (e.g. EMFILE) does not leak descriptors across iterations.
            close(fd)?;
            panic!("fork() failed at iteration {} (ret={})", i, ret);
        }

        let mut wstatus: c_int = 0;
        // SAFETY: `wstatus` is a valid `c_int`.
        let reaped: pid_t = unsafe { bindings::waitpid::waitpid(ret, &raw mut wstatus, 0) };
        assert!(
            reaped == ret,
            "waitpid() must reap the child at iteration {} (ret={}, child={})",
            i,
            reaped,
            ret
        );

        // Close the parent's copy of the descriptor before checking the result.
        close(fd)?;

        assert!(
            wifexited(wstatus) && wexitstatus(wstatus) == 0,
            "fork()+execv()'d child failed at iteration {} (status={:#x}); a descriptor opened \
             before fork() did not survive the child's execv()",
            i,
            wstatus
        );
    }

    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!("fork-exec-loop-test: starting fork()+execv() fd-inheritance test");

    test_fork_exec_loop()?;
    ::syslog::info!("fork-exec-loop-test: PASS - fork_exec_loop");

    // Magic string consumed by the CI harness to mark a successful run.
    let magic_string: &[u8] = b"ok";
    write(STDOUT_FILENO, magic_string)?;

    Ok(())
}
