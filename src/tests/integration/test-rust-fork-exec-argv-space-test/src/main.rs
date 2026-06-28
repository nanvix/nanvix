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

//! # `execv()` Argument Containing a Space Across `fork()` Regression Test (caller)
//!
//! Acceptance test: `execv()` must accept and deliver an argument that contains an embedded space.
//! POSIX places no restriction on the bytes of an argument other than the terminating NUL, so
//! `argv = ["target", "alpha beta"]` must reach the new image as exactly two arguments, the second
//! being the 10-byte string `"alpha beta"`.
//!
//! The caller forks; the child `execv()`s `/target` (`fork-exec-argv-space-target`) passing
//! [`SPACE_ARG`] (`"alpha beta"`) as `argv[1]`; the target checks `argv[1]` equals that string
//! exactly and exits `0`. The parent requires the child to exit `0`.
//!
//! On Nanvix today this FAILS: `execv()` flattens `argv` (and `envp`) into a single space-separated
//! string that the new image re-splits on spaces, so a token may not contain a space. The
//! implementation rejects such a token up front with `EINVAL` (see `validate_exec_token` /
//! `do_execv` in `syscall/src/unistd/exec/mod.rs`), so the child's `execv()` returns instead of
//! replacing the image and the child exits with [`CHILD_EXECV_FAILED`]. Even if the token were not
//! rejected, the space-separated wire format would split `"alpha beta"` into two arguments and the
//! target would observe `argv[1] == "alpha"`, also failing. This is exactly why a `fork()`+`execv()`'d
//! CPython cannot run `python -c "<code with spaces>"` -- e.g. `subprocess.run([... , "-c",
//! "print('x', y)"])` -- and fails with `OSError: [Errno 22]`.
//!
//! The bug is independent of guest memory size; it is a property of the argument-passing wire
//! format. While the bug is present the test FAILS; once `execv()` carries argument bytes verbatim
//! (including spaces) and delimits arguments unambiguously, it passes and guards the behavior.
//! `/target` is bundled into the test ramfs by the harness.

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

/// The argument carrying an embedded space. The target verifies it arrives verbatim as a single
/// argument. Must match fork-exec-argv-space-target.
const SPACE_ARG: &str = "alpha beta";

/// Exit status reported by the child if execv() returned (i.e. failed to replace the image).
const CHILD_EXECV_FAILED: c_int = 127;

//==================================================================================================
// Test
//==================================================================================================

/// Verifies that a fork()+execv()'d child receives an argument containing a space verbatim.
fn test_fork_exec_argv_space() -> Result<(), Error> {
    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        // Child: exec the target, passing a single argument that contains an embedded space.
        let _error: Error = do_execv(TARGET_PATH, &["target", SPACE_ARG], &[]);
        // Only reached if execv() itself failed (today: EINVAL because the argument has a space).
        // SAFETY: the child holds no resources requiring cleanup; terminate immediately.
        unsafe { bindings::_exit::_exit(CHILD_EXECV_FAILED) };
    }
    assert!(ret > 0, "fork() failed (ret={})", ret);

    let mut wstatus: c_int = 0;
    // SAFETY: `wstatus` is a valid `c_int`.
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(ret, &raw mut wstatus, 0) };
    assert!(reaped == ret, "waitpid() must reap the child (ret={}, child={})", reaped, ret);

    assert!(
        wifexited(wstatus) && wexitstatus(wstatus) == 0,
        "execv() did not deliver an argument containing a space verbatim (status={:#x}); the \
         space-separated argv wire format rejects or splits such arguments",
        wstatus
    );

    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!("fork-exec-argv-space-test: starting execv() space-argument test");

    test_fork_exec_argv_space()?;
    ::syslog::info!("fork-exec-argv-space-test: PASS - fork_exec_argv_space");

    // Magic string consumed by the CI harness to mark a successful run.
    let magic_string: &[u8] = b"ok";
    write(STDOUT_FILENO, magic_string)?;

    Ok(())
}
