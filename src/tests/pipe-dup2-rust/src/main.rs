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

//! # `dup2()` Pipe Redirection Regression Test
//!
//! Acceptance test for `dup2()` redirection of a standard stream (`nanvix/nanvix#354`).
//!
//! A `dup2(oldfd, newfd)` must make `newfd` refer to the same open file description as `oldfd`, so
//! that subsequent writes to `newfd` are delivered through `oldfd`'s object. This is the mechanism
//! shells and subprocess libraries use to redirect a child's standard streams (`pipe()` +
//! `dup2()` + `exec()`).
//!
//! The test forks; the child redirects its standard output onto the write end of a pipe with
//! `dup2()` and writes a marker; the parent reads the marker back from the pipe's read end. The
//! redirection happens in the child so the parent's own standard output (which carries the `"ok"`
//! sentinel) is never disturbed.
//!
//! If `dup2()` fails to redirect the stream the child exits non-zero and no marker reaches the
//! pipe, failing the test; a successful round-trip of the marker guards the redirection behavior
//! against regressions.

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
    dup2,
    pipe,
    read,
    write,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Marker the child writes to its redirected standard output and the parent reads from the pipe.
const MARKER: &[u8] = b"PIPE-DUP2-REDIRECT-OK";

/// Exit status reported by the child when `dup2()` did not redirect the stream.
const CHILD_DUP2_FAILED: c_int = 2;

//==================================================================================================
// Test
//==================================================================================================

/// Verifies that `dup2()` redirects a standard stream onto a pipe.
fn test_dup2_redirects_stdout() -> Result<(), Error> {
    let fds: [i32; 2] = pipe()?;
    let read_fd: i32 = fds[0];
    let write_fd: i32 = fds[1];

    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        // Child: redirect its standard output onto the pipe's write end, then write the marker.
        if dup2(write_fd, STDOUT_FILENO).is_err() {
            // dup2() did not redirect the stream. Report failure.
            // SAFETY: the child holds no resources requiring cleanup; terminate immediately.
            unsafe { bindings::_exit::_exit(CHILD_DUP2_FAILED) };
        }
        // The standard output now refers to the pipe's write end. Write the whole marker,
        // treating short writes and zero-length writes as failures so the parent-side assertion
        // only succeeds when the full marker was delivered.
        let mut written: usize = 0;
        let status: c_int = loop {
            match write(STDOUT_FILENO, &MARKER[written..]) {
                Ok(0) | Err(_) => break 1,
                Ok(count) => {
                    written += usize::try_from(count).unwrap_or(usize::MAX);
                    if written >= MARKER.len() {
                        break 0;
                    }
                },
            }
        };
        // SAFETY: as above.
        unsafe { bindings::_exit::_exit(status) };
    }
    assert!(ret > 0, "fork() failed (ret={})", ret);

    // Parent: close its own copy of the write end so the read terminates once the child is done.
    close(write_fd)?;

    // Pipe reads may return partial data, so loop until the full marker has been read or EOF is
    // reached before asserting.
    let mut buf: [u8; MARKER.len()] = [0u8; MARKER.len()];
    let mut n: usize = 0;
    while n < MARKER.len() {
        let count: usize = usize::try_from(read(read_fd, &mut buf[n..])?).unwrap_or(usize::MAX);
        if count == 0 {
            break; // EOF: the child closed (or never reached) the write end.
        }
        n += count;
    }

    let mut wstatus: c_int = 0;
    // SAFETY: `wstatus` is a valid `c_int`.
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(ret, &raw mut wstatus, 0) };
    assert!(reaped == ret, "waitpid() must reap the child (ret={}, child={})", reaped, ret);

    close(read_fd)?;

    assert!(
        wifexited(wstatus) && wexitstatus(wstatus) == 0,
        "child could not redirect standard output via dup2() (status={:#x}; nanvix/nanvix#354)",
        wstatus
    );
    assert!(
        n == MARKER.len() && buf == *MARKER,
        "marker written to the redirected standard output did not reach the pipe (read {} bytes)",
        n
    );

    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!("pipe-dup2-rust: starting dup2() pipe-redirection regression test");

    test_dup2_redirects_stdout()?;
    ::syslog::info!("pipe-dup2-rust: PASS - dup2_redirects_stdout");

    // Magic string consumed by the CI harness to mark a successful run.
    let magic_string: &[u8] = b"ok";
    write(STDOUT_FILENO, magic_string)?;

    Ok(())
}
