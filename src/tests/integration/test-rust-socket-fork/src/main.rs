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

//! # Socket Reference-Counting Across `fork()` Regression Test
//!
//! Acceptance test for `nanvix/nanvix#2609`: an `AF_INET` socket must be reference-counted across
//! `fork()`, so that a child closing its inherited descriptor does NOT destroy the parent's socket.
//!
//! POSIX semantics: `fork()` duplicates the parent's open socket descriptors; each holds an
//! independent reference to the same open socket description, which is released only when the LAST
//! descriptor referring to it is closed. Every mainstream OS (Linux, the BSDs, ...) behaves this
//! way, and forking servers rely on it.
//!
//! Nanvix today shares the socket rather than reference-counting it: `networkd` maps a guest socket
//! descriptor to a host socket by a fixed arithmetic offset with no per-process reference count, so
//! the first `close()` in EITHER process tears the socket down for both. Until that is fixed this
//! test FAILS at the final probe (the parent's `getsockname()` errors after the child's `close()`).
//! Once sockets are reference-counted across `fork()`, the test passes and guards the behavior.
//!
//! The test deliberately uses only the kernel-IKC standard output and a `waitpid()` rendezvous, so
//! it needs no filesystem and is deterministic.

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
use ::syscall::{
    netinet::in_::{
        Ipv4Addr,
        Protocol,
        SocketAddrV4,
    },
    sys::socket::{
        AddressFamily,
        SocketAddr,
        SocketType,
        syscall::{
            bind,
            getsockname,
            socket,
        },
    },
    unistd::{
        bindings,
        close,
        write,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Loopback address the test socket is bound to.
const LOOPBACK: [u8; 4] = [127, 0, 0, 1];

/// Port the test socket is bound to (mnemonic for issue #2609).
const BIND_PORT: u16 = 2609;

/// Exit status reported by the child when it successfully closed its inherited socket copy.
const CHILD_OK: c_int = 0;

/// Exit status reported by the child when closing its inherited socket copy failed.
const CHILD_CLOSE_FAILED: c_int = 1;

//==================================================================================================
// Helpers
//==================================================================================================

/// Probes whether a socket descriptor still refers to a live socket via `getsockname()`.
fn socket_alive(sockfd: c_int) -> bool {
    let mut name: SocketAddr = SocketAddr::V4(SocketAddrV4::default());
    getsockname(sockfd, &mut name).is_ok()
}

//==================================================================================================
// Test
//==================================================================================================

/// Verifies that a socket survives a child closing its inherited copy after `fork()`.
fn test_socket_refcounted_across_fork() -> Result<(), Error> {
    // Parent creates and binds an AF_INET stream socket.
    let addr: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(LOOPBACK), BIND_PORT));
    let sockfd: c_int = socket(AddressFamily::Inet, SocketType::Stream, Protocol::Tcp)?;
    bind(sockfd, &addr)?;

    // The socket is alive before fork().
    assert!(socket_alive(sockfd), "socket must be alive before fork()");

    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        // Child: close its inherited copy and exit. Under POSIX this only drops the child's
        // reference; the parent's descriptor stays open.
        let status: c_int = match close(sockfd) {
            Ok(()) => CHILD_OK,
            Err(_) => CHILD_CLOSE_FAILED,
        };
        // SAFETY: the child holds no resources requiring cleanup; terminate immediately.
        unsafe { bindings::_exit::_exit(status) };
    }
    assert!(ret > 0, "fork() failed (ret={})", ret);

    // Parent: wait for the child to finish closing its copy before probing.
    let mut wstatus: c_int = 0;
    // SAFETY: `wstatus` is a valid `c_int`.
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(ret, &raw mut wstatus, 0) };
    assert!(reaped == ret, "waitpid() must reap the child (ret={}, child={})", reaped, ret);
    assert!(
        wifexited(wstatus) && wexitstatus(wstatus) == CHILD_OK,
        "child failed to close its inherited socket copy (status={:#x})",
        wstatus
    );

    // The parent's socket must still be alive. If sockets are shared rather than reference-counted
    // across fork(), the child's close() destroyed it and getsockname() now fails (issue #2609).
    assert!(
        socket_alive(sockfd),
        "parent socket was destroyed by the child's close() (nanvix/nanvix#2609)"
    );

    close(sockfd)?;
    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!("test-rust-socket-fork: starting socket-across-fork regression test");

    test_socket_refcounted_across_fork()?;
    ::syslog::info!("test-rust-socket-fork: PASS - socket_refcounted_across_fork");

    // Magic string consumed by the CI harness to mark a successful run.
    let magic_string: &[u8] = b"ok";
    write(STDOUT_FILENO, magic_string)?;

    Ok(())
}
