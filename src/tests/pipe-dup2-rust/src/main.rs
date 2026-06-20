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

/// Probes the vfsd console terminal through `isatty` and the terminal-control `ioctl`s.
///
/// Exercises the standalone `isatty`/`ioctl` routing end to end: a console descriptor reports as a
/// terminal, a pipe end does not, an invalid descriptor fails with `EBADF`, the stored window size
/// and terminal attributes are returned, a `TCSETS` through one console descriptor is observed
/// through another (the shared terminal), and a terminal `ioctl` on a non-terminal fails with
/// `ENOTTY`. The original attributes are restored so the console is left as found.
#[cfg(feature = "standalone")]
fn test_tty_probing() -> Result<(), Error> {
    use ::sys::error::ErrorCode;
    use ::sysapi::{
        ffi::c_void,
        sys_ioctl::{
            TCGETS,
            TCSETS,
            TIOCGWINSZ,
            Winsize,
        },
        termios::{
            ECHO,
            ICANON,
            Termios,
            VMIN,
        },
        unistd::{
            STDERR_FILENO,
            STDIN_FILENO,
        },
    };
    use ::syscall::{
        sys::ioctl::ioctl,
        unistd::{
            dup,
            isatty,
        },
    };

    // The standard streams are terminals.
    for fd in [STDIN_FILENO, STDOUT_FILENO, STDERR_FILENO] {
        match isatty(fd) {
            Ok(true) => {},
            other => panic!("console descriptor {fd} must be a terminal (got {other:?})"),
        }
    }

    // A duplicate of a console descriptor is still a terminal.
    let dup_fd: c_int = dup(STDOUT_FILENO)?;
    match isatty(dup_fd) {
        Ok(true) => {},
        other => panic!("a duplicate of a console descriptor must be a terminal (got {other:?})"),
    }
    close(dup_fd)?;

    // An invalid descriptor fails with EBADF.
    match isatty(-1) {
        Err(error) if matches!(error.code, ErrorCode::BadFile) => {},
        other => panic!("isatty(-1) must fail with EBADF (got {other:?})"),
    }

    // A pipe end is not a terminal.
    let fds: [i32; 2] = pipe()?;
    let read_fd: i32 = fds[0];
    let write_fd: i32 = fds[1];
    match isatty(read_fd) {
        Ok(false) => {},
        other => panic!("a pipe end must not be a terminal (got {other:?})"),
    }

    // ioctl(TIOCGWINSZ) returns the stored default window size. Starting from a zeroed value proves
    // the buffer is actually filled by vfsd.
    let mut winsize: Winsize = Winsize::default();
    match unsafe {
        ioctl(STDIN_FILENO, TIOCGWINSZ, ::core::ptr::from_mut(&mut winsize).cast::<c_void>())
    } {
        Ok(0) => {},
        other => panic!("ioctl(TIOCGWINSZ) must succeed (got {other:?})"),
    }
    assert!(
        winsize == Winsize::console_default(),
        "ioctl(TIOCGWINSZ) must return the default window size (got {winsize:?})"
    );

    // TCGETS returns the default attributes (canonical mode with echo). The local flags are
    // clobbered first so a correct read must overwrite them.
    let mut attrs: Termios = Termios::console_default();
    attrs.c_lflag = 0;
    match unsafe { ioctl(STDIN_FILENO, TCGETS, ::core::ptr::from_mut(&mut attrs).cast::<c_void>()) }
    {
        Ok(0) => {},
        other => panic!("ioctl(TCGETS) must succeed (got {other:?})"),
    }
    assert!(attrs.c_lflag & ICANON != 0, "the default attributes must enable canonical mode");
    assert!(attrs.c_lflag & ECHO != 0, "the default attributes must enable echo");
    let saved: Termios = attrs;

    // A change made through stdout is observed through stdin: the standard streams share one
    // terminal. Clears canonical mode and echo and sets a distinctive VMIN.
    let mut modified: Termios = saved;
    modified.c_lflag &= !(ICANON | ECHO);
    modified.c_cc[VMIN] = 9;
    match unsafe {
        ioctl(STDOUT_FILENO, TCSETS, ::core::ptr::from_mut(&mut modified).cast::<c_void>())
    } {
        Ok(0) => {},
        other => panic!("ioctl(TCSETS) must succeed (got {other:?})"),
    }
    let mut readback: Termios = Termios::console_default();
    readback.c_cc[VMIN] = 0;
    match unsafe {
        ioctl(STDIN_FILENO, TCGETS, ::core::ptr::from_mut(&mut readback).cast::<c_void>())
    } {
        Ok(0) => {},
        other => panic!("ioctl(TCGETS) after a set must succeed (got {other:?})"),
    }
    assert!(
        readback.c_cc[VMIN] == 9,
        "a TCSETS through stdout must be observed through stdin (VMIN={})",
        readback.c_cc[VMIN]
    );
    assert!(
        readback.c_lflag & ICANON == 0,
        "the cleared canonical-mode flag must be observed through stdin"
    );

    // Restore the original attributes so the console is left as it was found.
    let mut restore: Termios = saved;
    match unsafe {
        ioctl(STDOUT_FILENO, TCSETS, ::core::ptr::from_mut(&mut restore).cast::<c_void>())
    } {
        Ok(0) => {},
        other => panic!("restoring the terminal attributes must succeed (got {other:?})"),
    }

    // A terminal ioctl on a non-terminal descriptor fails with ENOTTY.
    let mut probe: Termios = Termios::console_default();
    match unsafe { ioctl(read_fd, TCGETS, ::core::ptr::from_mut(&mut probe).cast::<c_void>()) } {
        Err(error) if matches!(error.code, ErrorCode::NotTerminal) => {},
        other => panic!("a terminal ioctl on a pipe must fail with ENOTTY (got {other:?})"),
    }

    close(read_fd)?;
    close(write_fd)?;

    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!("pipe-dup2-rust: starting dup2() pipe-redirection regression test");

    // Probe the vfsd console terminal (isatty/ioctl/termios). This exercises the standalone
    // syscall -> vfsd routing, so it only runs when that routing is compiled in.
    #[cfg(feature = "standalone")]
    {
        test_tty_probing()?;
        ::syslog::info!("pipe-dup2-rust: PASS - tty_probing");
    }

    test_dup2_redirects_stdout()?;
    ::syslog::info!("pipe-dup2-rust: PASS - dup2_redirects_stdout");

    // Magic string consumed by the CI harness to mark a successful run.
    let magic_string: &[u8] = b"ok";
    write(STDOUT_FILENO, magic_string)?;

    Ok(())
}
