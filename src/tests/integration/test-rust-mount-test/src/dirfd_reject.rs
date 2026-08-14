// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Invalid-`dirfd` rejection tests for the `*at()` operations routed over hostfs.
//!
//! When a relative path is paired with a `dirfd` that cannot be resolved to a VFS
//! directory (for example, a regular-file descriptor), vfsd must reject the request
//! with `ENOTDIR` (`InvalidDirectory`) instead of silently dropping the `dirfd` and
//! resolving the path against the current working directory.
//!
//! These tests cover the five operations whose hostfs handlers perform this
//! resolution: `openat`, `mkdirat`, `fstatat`, `symlinkat`, and `readlinkat`. The
//! rejection happens in vfsd before any host round-trip, so the assertions hold even
//! on hosts that do not support symbolic links.

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    fcntl::{
        atflags::AT_FDCWD,
        file_access_mode::{
            O_RDONLY,
            O_WRONLY,
        },
        file_creation_flags::{
            O_CREAT,
            O_TRUNC,
        },
    },
    ffi::c_int,
    sys_stat::{
        file_mode::{
            S_IRUSR,
            S_IRWXU,
            S_IWUSR,
        },
        stat,
    },
};

pub fn test() -> Result<(), Error> {
    test_invalid_dirfd_rejected()
}

/// Asserts that a result is the expected `ENOTDIR` rejection, panicking otherwise.
fn expect_reject<T>(op: &str, result: Result<T, Error>) {
    match result {
        Ok(_) => {
            panic!(
                "dirfd-reject: {op} with a non-directory dirfd must fail with ENOTDIR, but it \
                 succeeded"
            );
        },
        Err(e) if e.code == ErrorCode::InvalidDirectory => {
            ::syslog::info!("mount-test: [PASS] {op} rejects a non-directory dirfd with ENOTDIR");
        },
        Err(e) => {
            let code: ErrorCode = e.code;
            panic!(
                "dirfd-reject: {op} with a non-directory dirfd must fail with ENOTDIR, got \
                 {code:?}"
            );
        },
    }
}

/// Tests that the hostfs-routed `*at()` operations reject a relative path paired
/// with a non-directory `dirfd` instead of falling back to the cwd.
fn test_invalid_dirfd_rejected() -> Result<(), Error> {
    let probe_path: &str = "/mnt/dirfd-reject-probe.txt";
    let mode: c_int = (S_IRUSR | S_IWUSR) as c_int;

    // Obtain a non-directory descriptor by creating a regular file on the mount.
    // Using this file descriptor as a `dirfd` must be rejected by every `*at()`
    // operation below.
    let file_fd: c_int =
        ::syscall::fcntl::openat(AT_FDCWD, probe_path, O_CREAT | O_WRONLY | O_TRUNC, mode as u32)?;

    // openat: a stray success would leak a descriptor, so close it before failing.
    match ::syscall::fcntl::openat(file_fd, "reject-open.txt", O_RDONLY, 0) {
        Ok(stray) => {
            let _ = ::syscall::unistd::close(stray);
            panic!(
                "dirfd-reject: openat with a non-directory dirfd must fail with ENOTDIR, but it \
                 succeeded"
            );
        },
        Err(e) if e.code == ErrorCode::InvalidDirectory => {
            ::syslog::info!("mount-test: [PASS] openat rejects a non-directory dirfd with ENOTDIR");
        },
        Err(e) => {
            let code: ErrorCode = e.code;
            panic!(
                "dirfd-reject: openat with a non-directory dirfd must fail with ENOTDIR, got \
                 {code:?}"
            );
        },
    }

    expect_reject("mkdirat", ::syscall::sys::stat::mkdirat(file_fd, "reject-dir", S_IRWXU));

    let mut st: stat = stat::default();
    expect_reject("fstatat", ::syscall::sys::stat::fstatat(file_fd, "reject-stat.txt", &mut st, 0));

    // symlinkat and readlinkat previously reported "not supported" for an
    // unresolvable dirfd; they must now report ENOTDIR like the other operations.
    expect_reject(
        "symlinkat",
        ::syscall::unistd::symlinkat("target.txt", file_fd, "reject-link.lnk"),
    );

    let mut buf: [u8; 64] = [0u8; 64];
    expect_reject(
        "readlinkat",
        ::syscall::unistd::readlinkat(file_fd, "reject-link.lnk", &mut buf),
    );

    // Clean up the probe file.
    ::syscall::unistd::close(file_fd)?;
    ::syscall::fcntl::unlinkat(AT_FDCWD, probe_path, 0)?;

    Ok(())
}
