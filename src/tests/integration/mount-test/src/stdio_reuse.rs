// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Integration tests proving that closing a standard descriptor releases its slot in `vfsd`'s flat
//! descriptor table, so the freed number becomes the lowest free descriptor and is handed back to a
//! later `open()`.
//!
//! Under the flat namespace a descriptor's number no longer encodes its backend: `0`/`1`/`2` are
//! seeded as console descriptors in `vfsd` for a normally-launched process, but they live in the
//! same per-process table as every other descriptor. Closing one must therefore free its number for
//! reuse, exactly like closing any regular file. These tests exercise that end-to-end through
//! libposix and `vfsd`.
//!
//! `stdout` (`1`) is deliberately left untouched: the harness needs it for the success sentinel.
//! Only `stdin` (`0`) and `stderr` (`2`) are closed, and both are restored to a free state before
//! returning. Guest `syslog` writes through the kernel debug channel, not `stderr`, so closing `2`
//! does not affect diagnostics.

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    fcntl::{
        file_access_mode::O_RDWR,
        file_creation_flags::{
            O_CREAT,
            O_TRUNC,
        },
    },
    ffi::c_int,
    sys_types::mode_t,
    unistd::{
        file_seek::SEEK_SET,
        STDERR_FILENO,
        STDIN_FILENO,
    },
};
use ::syscall::{
    fcntl,
    unistd,
};

/// Permission bits for a user read/write scratch file (`0o600`).
const FILE_MODE: mode_t = 0o600;

/// Open flags for a truncating read/write scratch file.
const SCRATCH_FLAGS: c_int = O_RDWR | O_CREAT | O_TRUNC;

/// Runs the standard-descriptor close/reuse tests.
pub fn test() -> Result<(), Error> {
    test_close_stdin_reuses_zero()?;
    test_close_stderr_reuses_two()?;
    ::syslog::info!("mount-test: [PASS] stdio descriptor close/reuse");
    Ok(())
}

/// Closing `stdin` frees descriptor `0`; the next `open()` must hand it back, and the reused
/// descriptor must behave as a real file rather than a stale console token.
fn test_close_stdin_reuses_zero() -> Result<(), Error> {
    const PATH: &str = "/mnt/stdio-reuse-stdin.txt";
    const DATA: &[u8] = b"reuse-stdin";

    // `stdin` starts open as a console descriptor seeded by `vfsd`; closing it releases slot `0`.
    unistd::close(STDIN_FILENO)?;

    // With `0` released, it is the lowest free descriptor, so the next `open()` must reuse it.
    let fd: c_int = fcntl::open(PATH, SCRATCH_FLAGS, FILE_MODE)?;
    if fd != STDIN_FILENO {
        ::syslog::error!("mount-test: expected closed stdin to be reused as fd 0, got {fd}");
        return Err(Error::new(ErrorCode::InvalidArgument, "closed stdin was not reused as fd 0"));
    }

    // The reused descriptor must be a fully functional file: write, rewind, and read back.
    let written: usize = unistd::write(fd, DATA)? as usize;
    if written != DATA.len() {
        return Err(Error::new(ErrorCode::IoErr, "short write on the reused descriptor"));
    }
    unistd::lseek(fd, 0, SEEK_SET)?;
    let mut buf: [u8; 16] = [0u8; 16];
    let read: usize = unistd::read(fd, &mut buf)? as usize;
    if &buf[..read] != DATA {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "read-back through the reused descriptor mismatched",
        ));
    }

    // Release the reused descriptor (freeing `0` again) and remove the scratch file.
    unistd::close(fd)?;
    unistd::unlink(PATH)?;

    ::syslog::info!("mount-test: [PASS] close(stdin) frees fd 0 for reuse");
    Ok(())
}

/// Closing `stderr` frees descriptor `2`; with `0` and `1` held, the next `open()` must hand back
/// exactly `2`. This proves a freed standard descriptor is reused by its own number, not merely by
/// the lowest one overall.
fn test_close_stderr_reuses_two() -> Result<(), Error> {
    const OCCUPY_PATH: &str = "/mnt/stdio-reuse-occupy.txt";
    const PATH: &str = "/mnt/stdio-reuse-stderr.txt";

    // Hold descriptor `0` (freed by the previous test) so that, together with `stdout` at `1`, the
    // only lower number left free once `stderr` is closed is `2` itself.
    let occupy_fd: c_int = fcntl::open(OCCUPY_PATH, SCRATCH_FLAGS, FILE_MODE)?;
    if occupy_fd != STDIN_FILENO {
        ::syslog::error!("mount-test: expected occupying open to take fd 0, got {occupy_fd}");
        return Err(Error::new(ErrorCode::InvalidArgument, "occupying open did not take fd 0"));
    }

    // `stderr` is still a console descriptor seeded by `vfsd`; closing it releases slot `2`.
    unistd::close(STDERR_FILENO)?;

    // With `0` and `1` held, descriptor `2` is now the lowest free number and must be reused.
    let fd: c_int = fcntl::open(PATH, SCRATCH_FLAGS, FILE_MODE)?;
    if fd != STDERR_FILENO {
        ::syslog::error!("mount-test: expected closed stderr to be reused as fd 2, got {fd}");
        return Err(Error::new(ErrorCode::InvalidArgument, "closed stderr was not reused as fd 2"));
    }

    // Release both descriptors (freeing `0` and `2` again) and remove the scratch files.
    unistd::close(fd)?;
    unistd::close(occupy_fd)?;
    unistd::unlink(PATH)?;
    unistd::unlink(OCCUPY_PATH)?;

    ::syslog::info!("mount-test: [PASS] close(stderr) frees fd 2 for reuse");
    Ok(())
}
