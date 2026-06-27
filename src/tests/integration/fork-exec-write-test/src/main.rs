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

//! # Filesystem Write Visibility After `fork()` + `execv()` Regression Test (caller)
//!
//! Acceptance test: a file written by a `fork()`+`execv()`'d child must be visible to the parent,
//! because the two processes share the filesystem and because a returned `write()` is visible to
//! other readers per POSIX -- regardless of whether the writer `close()`s the file or terminates
//! via `_exit()`.
//!
//! The exec'd child (`fork-exec-write-target`) creates `/exec_write.out`, `write()`s a known
//! payload, and then `_exit()`s WITHOUT `close()`. On Nanvix today such a write is buffered
//! per-process and only committed to vfsd on `close()` or the normal C-runtime shutdown flush, so a
//! child that exits without closing loses its data: the parent sees the file created but EMPTY (a
//! short read of zero bytes). This breaks the standard "fork, exec a helper that produces an output
//! file, then read that file" workflow -- e.g. running a script in a separate interpreter that
//! terminates via `_exit()` and collecting its output.
//!
//! The caller forks; the child `execv()`s `/target`; after `waitpid()`, the parent opens
//! `/exec_write.out` and checks the payload. While the bug is present the file is empty (or missing)
//! in the parent's view and the test FAILS; once a returned `write()` is committed independently of
//! `close()`, the parent reads the payload back and the test passes.
//!
//! `/exec_write.out` is NOT pre-seeded; the target creates it. The target is bundled at `/target`
//! by the test harness (see the standalone image wiring).

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

use ::sys::error::{
    Error,
    ErrorCode,
};
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
        read,
        write,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Path of the execv() target in the mounted ramfs (mounted at the filesystem root).
const TARGET_PATH: &str = "/target";

/// Path the target writes; the caller reads it back. Must match fork-exec-write-target.
const OUTPUT_PATH: &str = "/exec_write.out";

/// Payload the target writes. Must match fork-exec-write-target.
const PAYLOAD: &[u8] = b"FORK-EXEC-WRITE-PAYLOAD";

/// Exit status reported by the child if execv() returned (i.e. failed).
const CHILD_EXECV_FAILED: c_int = 127;

//==================================================================================================
// Test
//==================================================================================================

/// Verifies that a file written by a fork()+execv()'d child is visible to the parent.
fn test_fork_exec_write_visible() -> Result<(), Error> {
    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        // Child: replace the image with the target, which writes OUTPUT_PATH and _exit()s.
        let _error: Error = do_execv(TARGET_PATH, &["target"], &[]);
        // Only reached if execv() itself failed.
        // SAFETY: the child holds no resources requiring cleanup; terminate immediately.
        unsafe { bindings::_exit::_exit(CHILD_EXECV_FAILED) };
    }
    assert!(ret > 0, "fork() failed (ret={})", ret);

    // Parent: wait for the exec'd child to finish writing the file.
    let mut wstatus: c_int = 0;
    // SAFETY: `wstatus` is a valid `c_int`.
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(ret, &raw mut wstatus, 0) };
    assert!(reaped == ret, "waitpid() must reap the child (ret={}, child={})", reaped, ret);
    assert!(
        wifexited(wstatus) && wexitstatus(wstatus) == 0,
        "fork()+execv()'d child failed to write its output file (status={:#x})",
        wstatus
    );

    // The file the child wrote must be visible to the parent, with the expected contents. While
    // writes are committed only on close()/runtime flush, the child's _exit() loses the data and
    // the parent reads zero bytes.
    let mode: mode_t = 0;
    let fd: c_int = open(OUTPUT_PATH, O_RDONLY, mode)?;

    let mut buf: [u8; PAYLOAD.len()] = [0u8; PAYLOAD.len()];
    let n: usize = usize::try_from(read(fd, &mut buf)?)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "invalid read length"))?;
    close(fd)?;

    assert!(
        n == PAYLOAD.len() && buf.as_slice() == PAYLOAD,
        "file written by the fork()+execv()'d child is not visible to the parent (read {} bytes)",
        n
    );

    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!("fork-exec-write-test: starting fork()+execv() write-visibility test");

    test_fork_exec_write_visible()?;
    ::syslog::info!("fork-exec-write-test: PASS - fork_exec_write_visible");

    // Magic string consumed by the CI harness to mark a successful run.
    let magic_string: &[u8] = b"ok";
    write(STDOUT_FILENO, magic_string)?;

    Ok(())
}
