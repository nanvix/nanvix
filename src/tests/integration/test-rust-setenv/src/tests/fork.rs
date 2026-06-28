// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # `getenv()` / `setenv()` / `unsetenv()` Across `fork()`
//!
//! Exercises the POSIX environment API across `fork()` to confirm that the per-process environment
//! table follows `fork()` semantics: the child receives an independent copy of the parent's
//! environment, and neither process observes the other's subsequent mutations.
//!
//! - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fork.html>
//! - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/setenv.html>
//! - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/unsetenv.html>
//! - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getenv.html>
//!
//! Three scenarios are covered, each deterministic:
//!
//! 1. **Inheritance** — a variable set by the parent before `fork()` is visible to the child.
//! 2. **Child isolation** — a child's `setenv()`/`unsetenv()` mutations do not affect the parent.
//! 3. **Parent isolation** — a parent's mutations *after* `fork()` are not visible to the child.
//!    An IPC barrier releases the child only after the parent has mutated its environment, turning
//!    the ordering into a deterministic assertion rather than a timing-dependent flake.
//!
//! Each child reports its verdict through its exit status, which the parent reaps with `waitpid()`.

//==================================================================================================
// Imports
//==================================================================================================

use super::setenv::{
    do_getenv,
    do_setenv,
    do_unsetenv,
    getenv_is,
};
use ::sys::{
    error::Error,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    kcall::{
        ipc,
        pm,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::sysapi::{
    ffi::c_int,
    sys_types::pid_t,
    sys_wait::{
        wexitstatus,
        wifexited,
    },
};
use ::syscall::unistd::bindings;

//==================================================================================================
// Constants
//==================================================================================================

/// Non-zero `overwrite` flag for `setenv()` (replace an existing value).
const OVERWRITE: c_int = 1;

/// Exit status reported by a child when all of its checks pass.
const CHILD_OK: c_int = 0;

/// Exit status reported by a child when one of its checks fails.
const CHILD_FAIL: c_int = 1;

//==================================================================================================
// Helpers
//==================================================================================================

/// Reaps `child` with a blocking `waitpid()` and returns its decoded exit status.
fn reap_child(child: ProcessIdentifier) -> Result<c_int, Error> {
    let mut status: c_int = 0;
    // SAFETY: `status` is a valid, writable `c_int`.
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(i32::from(child), &raw mut status, 0) };
    assert!(
        reaped == i32::from(child),
        "waitpid() returned an unexpected pid (got={}, expected={})",
        reaped,
        i32::from(child)
    );
    assert!(wifexited(status), "child did not terminate normally (status={:#x})", status);
    Ok(wexitstatus(status))
}

/// Releases a child blocked on [`ipc::__kcall_recv`] by sending it an empty IPC message.
fn release_child(parent: ProcessIdentifier, child: ProcessIdentifier) -> Result<(), Error> {
    let go: Message = Message::new(
        MessageSender::new(parent, ThreadIdentifier::NONE),
        MessageReceiver::new(child, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        [0u8; Message::PAYLOAD_SIZE],
    );
    ipc::__kcall_send(&go)?;
    Ok(())
}

//==================================================================================================
// Child Paths
//==================================================================================================

/// Child body for the child-isolation scenario: mutates its own copy of the environment and
/// confirms it observes its own changes.
fn child_mutates_environment() -> c_int {
    let set_overwrite: bool = do_setenv(b"FORK_ISO\0", b"child_value\0", OVERWRITE) == 0;
    let set_new: bool = do_setenv(b"FORK_ISO_CHILD_ONLY\0", b"child_only\0", OVERWRITE) == 0;
    let removed: bool = do_unsetenv(b"FORK_ISO_DEL\0") == 0;

    let sees_overwrite: bool = getenv_is(b"FORK_ISO\0", b"child_value");
    let sees_new: bool = getenv_is(b"FORK_ISO_CHILD_ONLY\0", b"child_only");
    let sees_removed: bool = do_getenv(b"FORK_ISO_DEL\0").is_null();

    if set_overwrite && set_new && removed && sees_overwrite && sees_new && sees_removed {
        CHILD_OK
    } else {
        CHILD_FAIL
    }
}

/// Child body for the parent-isolation scenario: blocks on the IPC barrier until the parent has
/// mutated its own environment, then confirms the child's copy still reflects the pre-fork
/// snapshot.
fn child_observes_pre_fork_snapshot() -> c_int {
    // Block until the parent releases us; by then it has mutated its own environment.
    if ipc::__kcall_recv().is_err() {
        return CHILD_FAIL;
    }

    let unchanged: bool = getenv_is(b"FORK_PMOD\0", b"original");
    let no_leak: bool = do_getenv(b"FORK_PARENT_ONLY\0").is_null();

    if unchanged && no_leak {
        CHILD_OK
    } else {
        CHILD_FAIL
    }
}

//==================================================================================================
// Tests
//==================================================================================================

/// Verifies that a variable set by the parent before `fork()` is inherited by the child.
fn test_child_inherits_environment() -> Result<(), Error> {
    assert!(
        do_setenv(b"FORK_INHERIT\0", b"parent_value\0", OVERWRITE) == 0,
        "setenv() failed before fork()"
    );

    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        let code: c_int = if getenv_is(b"FORK_INHERIT\0", b"parent_value") {
            CHILD_OK
        } else {
            CHILD_FAIL
        };
        // SAFETY: the child holds no resources requiring cleanup; terminate immediately.
        unsafe { bindings::_exit::_exit(code) };
    }

    assert!(ret > 0, "fork() failed in parent (ret={})", ret);
    let child: ProcessIdentifier = ProcessIdentifier::from(ret);
    let code: c_int = reap_child(child)?;
    assert!(code == CHILD_OK, "child did not inherit the parent's variable (code={})", code);
    Ok(())
}

/// Verifies that a child's `setenv()`/`unsetenv()` mutations do not leak into the parent.
fn test_child_mutations_isolated() -> Result<(), Error> {
    assert!(do_setenv(b"FORK_ISO\0", b"parent_value\0", OVERWRITE) == 0, "setenv() failed");
    assert!(
        do_setenv(b"FORK_ISO_DEL\0", b"parent_keeps_this\0", OVERWRITE) == 0,
        "setenv() failed"
    );

    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        let code: c_int = child_mutates_environment();
        // SAFETY: the child holds no resources requiring cleanup; terminate immediately.
        unsafe { bindings::_exit::_exit(code) };
    }

    assert!(ret > 0, "fork() failed in parent (ret={})", ret);
    let child: ProcessIdentifier = ProcessIdentifier::from(ret);
    let code: c_int = reap_child(child)?;
    assert!(code == CHILD_OK, "child failed to mutate its own environment (code={})", code);

    // The parent's environment must be untouched by the child's mutations.
    assert!(getenv_is(b"FORK_ISO\0", b"parent_value"), "child's setenv() leaked into the parent");
    assert!(
        do_getenv(b"FORK_ISO_CHILD_ONLY\0").is_null(),
        "child's new variable leaked into the parent"
    );
    assert!(
        getenv_is(b"FORK_ISO_DEL\0", b"parent_keeps_this"),
        "child's unsetenv() leaked into the parent"
    );
    Ok(())
}

/// Verifies that a parent's mutations after `fork()` are not visible to the child.
fn test_parent_mutations_isolated() -> Result<(), Error> {
    let parent: ProcessIdentifier = pm::getpid_uncached()?;
    assert!(
        do_setenv(b"FORK_PMOD\0", b"original\0", OVERWRITE) == 0,
        "setenv() failed before fork()"
    );

    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        let code: c_int = child_observes_pre_fork_snapshot();
        // SAFETY: the child holds no resources requiring cleanup; terminate immediately.
        unsafe { bindings::_exit::_exit(code) };
    }

    assert!(ret > 0, "fork() failed in parent (ret={})", ret);
    let child: ProcessIdentifier = ProcessIdentifier::from(ret);

    // Mutate the parent's environment *after* the fork. The child is still blocked on its barrier.
    assert!(
        do_setenv(b"FORK_PMOD\0", b"changed\0", OVERWRITE) == 0,
        "setenv() failed after fork()"
    );
    assert!(
        do_setenv(b"FORK_PARENT_ONLY\0", b"parent_only\0", OVERWRITE) == 0,
        "setenv() failed after fork()"
    );

    // Release the child so its reads happen-after the parent's mutations above.
    release_child(parent, child)?;

    let code: c_int = reap_child(child)?;
    assert!(code == CHILD_OK, "child observed the parent's post-fork mutations (code={})", code);

    // Sanity: the parent observes its own mutations.
    assert!(getenv_is(b"FORK_PMOD\0", b"changed"), "parent lost its own mutation");
    assert!(getenv_is(b"FORK_PARENT_ONLY\0", b"parent_only"), "parent lost its own new variable");
    Ok(())
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// Runs all `fork()`-based environment-variable tests.
pub fn run() -> Result<(), Error> {
    ::syslog::info!("test-rust-setenv: starting fork() environment isolation tests");

    test_child_inherits_environment()?;
    ::syslog::info!("test-rust-setenv: PASS - child_inherits_environment");

    test_child_mutations_isolated()?;
    ::syslog::info!("test-rust-setenv: PASS - child_mutations_isolated");

    test_parent_mutations_isolated()?;
    ::syslog::info!("test-rust-setenv: PASS - parent_mutations_isolated");

    Ok(())
}
