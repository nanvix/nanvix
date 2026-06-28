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

//! # Bulk Pipe Data Integrity Across `fork()` + `execv()` Regression Test (caller)
//!
//! Acceptance test: when a `fork()`+`execv()`'d child streams a large payload to a pipe whose write
//! end was put on its standard output with `dup2()`, the parent reading the other end must receive
//! every byte. This is the `pipe()` + `dup2()` + `exec()` capture mechanism shells and subprocess
//! libraries use to collect a child's output.
//!
//! Each iteration the parent creates a pipe, forks, and the child redirects its standard output onto
//! the write end with `dup2()` and `execv()`s `/target` (`fork-exec-pipe-bulk-target`). The target
//! is a deliberately *large* image (its data segment is inflated to `MEMORY_SIZE / 8`, like
//! `execv-big-target`) that writes exactly [`TOTAL_BYTES`] bytes of the constant byte [`PATTERN`] to
//! its standard output and exits `0`. The parent drains the read end until end-of-file and checks it
//! received all [`TOTAL_BYTES`] bytes intact.
//!
//! On Nanvix today a *small* write through such an inherited, `dup2()`'d pipe is delivered (see
//! `pipe-dup2-rust`), and so is a bulk transfer from a *small* exec'd image. But a bulk transfer
//! from a *large* exec'd image does not reach the parent: the writer's `write()` to the pipe fails
//! part-way (the target exits non-zero) or, equivalently, the reader observes only a short prefix
//! and a premature end-of-file. The shortfall grows worse on each successive iteration, indicating a
//! per-cycle leak of pipe/descriptor state. This is the configuration a real `fork()`+`execv()`'d
//! interpreter hits when it streams a sizeable result back to a long-running parent through a
//! captured standard stream; a small helper does not trigger it, so the target is inflated on
//! purpose to reproduce it.
//!
//! The caller requires EVERY iteration to deliver the full payload; the first that comes up short
//! (or whose child exits non-zero) fails the test. While the bug is present the test FAILS; once a
//! bulk stream survives `fork()`+`execv()` of a large image intact it passes and guards the
//! behavior. `/target` is bundled into the test ramfs by the harness.

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
    close,
    do_execv,
    dup2,
    pipe,
    read,
    write,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Path of the execv() target in the mounted ramfs (mounted at the filesystem root).
const TARGET_PATH: &str = "/target";

/// Total number of bytes the target writes and the caller expects to read back. Must match
/// fork-exec-pipe-bulk-target.
const TOTAL_BYTES: usize = 1 << 20;

/// The constant byte the target writes `TOTAL_BYTES` times. Must match fork-exec-pipe-bulk-target.
const PATTERN: u8 = 0xA5;

/// Size of the stack buffer used to drain the pipe.
const CHUNK: usize = 4096;

/// Number of fork()+execv()+capture cycles to perform. The bug reproduces on the first cycle and
/// worsens with each one; the loop guards against a partial fix that only works some of the time.
const ITERATIONS: usize = 8;

/// Exit status reported by the child when `dup2()` did not redirect standard output.
const CHILD_DUP2_FAILED: c_int = 2;

/// Exit status reported by the child if execv() returned (i.e. failed).
const CHILD_EXECV_FAILED: c_int = 127;

//==================================================================================================
// Test
//==================================================================================================

/// Drains `read_fd` until end-of-file, returning the number of bytes read and whether every byte
/// equalled [`PATTERN`].
fn drain(read_fd: c_int) -> Result<(usize, bool), Error> {
    let mut buf: [u8; CHUNK] = [0u8; CHUNK];
    let mut total: usize = 0;
    let mut intact: bool = true;
    loop {
        let count: usize = usize::try_from(read(read_fd, &mut buf)?).unwrap_or(0);
        if count == 0 {
            break; // EOF: all write ends are closed.
        }
        if buf[..count].iter().any(|&b| b != PATTERN) {
            intact = false;
        }
        total += count;
    }
    Ok((total, intact))
}

/// Verifies that a bulk payload streamed by a fork()+execv()'d child through a dup2()'d pipe reaches
/// the parent in full, repeatedly.
fn test_fork_exec_pipe_bulk() -> Result<(), Error> {
    for i in 0..ITERATIONS {
        let fds: [i32; 2] = pipe()?;
        let read_fd: c_int = fds[0];
        let write_fd: c_int = fds[1];

        let ret: pid_t = bindings::fork::fork();
        if ret == 0 {
            // Child: redirect standard output onto the pipe's write end, then exec the target.
            if dup2(write_fd, STDOUT_FILENO).is_err() {
                // SAFETY: the child holds no resources requiring cleanup; terminate immediately.
                unsafe { bindings::_exit::_exit(CHILD_DUP2_FAILED) };
            }
            // Standard output now refers to the pipe; the inherited write ends close on exit, so the
            // parent still observes end-of-file once this child terminates.
            let _error: Error = do_execv(TARGET_PATH, &["target"], &[]);
            // Only reached if execv() itself failed.
            // SAFETY: as above.
            unsafe { bindings::_exit::_exit(CHILD_EXECV_FAILED) };
        }
        if ret < 0 {
            close(read_fd)?;
            close(write_fd)?;
            panic!("fork() failed at iteration {} (ret={})", i, ret);
        }

        // Parent: close its own copy of the write end so the read terminates once the child is done,
        // then drain the entire stream before reaping the child.
        close(write_fd)?;
        let (total, intact): (usize, bool) = drain(read_fd)?;
        close(read_fd)?;

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
        assert!(
            wifexited(wstatus) && wexitstatus(wstatus) == 0,
            "fork()+execv()'d child failed to stream its payload at iteration {} (status={:#x})",
            i,
            wstatus
        );
        assert!(
            total == TOTAL_BYTES && intact,
            "bulk payload streamed by the fork()+execv()'d child was truncated at iteration {}: \
             read {} of {} bytes (intact={})",
            i,
            total,
            TOTAL_BYTES,
            intact
        );
    }

    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!(
        "fork-exec-pipe-bulk-test: starting bulk pipe integrity test across fork()+execv()"
    );

    test_fork_exec_pipe_bulk()?;
    ::syslog::info!("fork-exec-pipe-bulk-test: PASS - fork_exec_pipe_bulk");

    // Magic string consumed by the CI harness to mark a successful run.
    let magic_string: &[u8] = b"ok";
    write(STDOUT_FILENO, magic_string)?;

    Ok(())
}
