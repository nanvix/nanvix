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

//! # Repeated Pipe-Capture Reliability Across `fork()` + `execv()` Regression Test (caller)
//!
//! Acceptance test: capturing a `fork()`+`execv()`'d child's output through a `dup2()`'d pipe must
//! work *every* time, not just once. A long-running parent (e.g. an HTTP server) spawns interpreter
//! subprocesses repeatedly; each capture must be reliable.
//!
//! This repeats a moderate ([`PAYLOAD_BYTES`]) capture [`ITERATIONS`] times and requires the full
//! payload back on *every* cycle. The exec'd `/target` (`fork-exec-pipe-loop-target`) is a
//! deliberately *large* image (data segment inflated to `MEMORY_SIZE / 8`, like `execv-big-target`)
//! so the repeated capture is exercised against the large-`execv()` path, which a long-running
//! parent hits when it spawns big interpreter subprocesses. Each cycle the parent pipes, forks, the
//! child `dup2()`s the write end onto standard output and `execv()`s `/target`, and the parent
//! drains the read end and checks it received all [`PAYLOAD_BYTES`] bytes intact with the child
//! exiting `0`. It records the smallest delivery across all cycles and fails if any cycle did not
//! deliver the full payload (or its child exited non-zero).
//!
//! The exec target must be stripped (see its `build.rs`): `execv()` requires the caller to mmap the
//! whole ELF image, so unstripped debug sections would inflate the file far beyond its loadable
//! size and exhaust physical memory before the image is even loaded.
//!
//! Because loading a large image many times is prohibitively slow under a trace-enabled (`debug`)
//! build -- the kernel emits per-page trace over a byte-at-a-time UART -- this test is gated to
//! `release` builds via `build_modes` in the test configuration. `/target` is bundled into the test
//! ramfs by the harness.

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

/// Bytes the target writes -- and the caller expects to read back -- on each cycle. Must match
/// fork-exec-pipe-loop-target. 64 KiB exceeds the pipe buffer, so each cycle must stream the data.
const PAYLOAD_BYTES: usize = 64 * 1024;

/// The constant byte the target writes. Must match fork-exec-pipe-loop-target.
const PATTERN: u8 = 0x5A;

/// Size of the stack buffer used to drain the pipe.
const CHUNK: usize = 4096;

/// Number of fork()+execv()+capture cycles to perform. A large count makes a flaky pass vanishingly
/// unlikely: every cycle must deliver the full payload, so any per-cycle regression in pipe or
/// descriptor handling is caught.
const ITERATIONS: usize = 64;

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

/// Runs one fork()+execv()+capture cycle, returning the number of bytes the parent received, whether
/// the bytes were intact, and whether the child exited 0.
fn one_cycle() -> Result<(usize, bool, bool), Error> {
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
        let _error: Error = do_execv(TARGET_PATH, &["target"], &[]);
        // SAFETY: as above.
        unsafe { bindings::_exit::_exit(CHILD_EXECV_FAILED) };
    }
    if ret < 0 {
        close(read_fd)?;
        close(write_fd)?;
        return Err(Error::new(::sys::error::ErrorCode::TryAgain, "fork() failed"));
    }

    // Parent: close its own copy of the write end so the read terminates once the child is done,
    // then drain the entire stream before reaping the child.
    close(write_fd)?;
    let (total, intact): (usize, bool) = drain(read_fd)?;
    close(read_fd)?;

    let mut wstatus: c_int = 0;
    // SAFETY: `wstatus` is a valid `c_int`.
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(ret, &raw mut wstatus, 0) };
    assert!(reaped == ret, "waitpid() must reap the child (ret={}, child={})", reaped, ret);

    let child_ok: bool = wifexited(wstatus) && wexitstatus(wstatus) == 0;
    Ok((total, intact, child_ok))
}

/// Verifies that repeated fork()+execv() pipe captures from a large image each deliver the full
/// payload.
fn test_fork_exec_pipe_loop() -> Result<(), Error> {
    let mut min_delivered: usize = usize::MAX;
    let mut all_intact: bool = true;
    let mut all_children_ok: bool = true;

    for i in 0..ITERATIONS {
        let (total, intact, child_ok): (usize, bool, bool) = one_cycle()?;
        if total < min_delivered {
            min_delivered = total;
        }
        all_intact = all_intact && intact;
        all_children_ok = all_children_ok && child_ok;

        // Fail as soon as a cycle comes up short so the failing cycle index is reported.
        assert!(
            total == PAYLOAD_BYTES && intact && child_ok,
            "repeated pipe capture across fork()+execv() failed at cycle {}: delivered {} of {} \
             bytes (intact={}, child_ok={}); capturing a forked+exec'd large image's pipe output \
             must be reliable on every cycle",
            i,
            total,
            PAYLOAD_BYTES,
            intact,
            child_ok
        );
    }

    assert!(
        min_delivered == PAYLOAD_BYTES && all_intact && all_children_ok,
        "across {} cycles the smallest delivery was {} of {} bytes (all_intact={}, \
         all_children_ok={})",
        ITERATIONS,
        min_delivered,
        PAYLOAD_BYTES,
        all_intact,
        all_children_ok
    );

    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!("fork-exec-pipe-loop-test: starting repeated pipe-capture reliability test");

    test_fork_exec_pipe_loop()?;
    ::syslog::info!("fork-exec-pipe-loop-test: PASS - fork_exec_pipe_loop");

    // Magic string consumed by the CI harness to mark a successful run.
    let magic_string: &[u8] = b"ok";
    write(STDOUT_FILENO, magic_string)?;

    Ok(())
}
