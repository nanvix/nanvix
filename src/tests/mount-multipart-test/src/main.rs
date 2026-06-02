// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Regression test for nanvix issue #2401.
//!
//! Reproduces the host-side hang introduced in commit `86c6c3f188` / PR #2388
//! ("[vfs] Fix path length limitation").
//!
//! The bug report observed the hang via the `nanvix-python` smoke test, which
//! happens to take a WHP snapshot first. The snapshot is incidental: the routing
//! bug fires whenever the host is launched without a `-mount` argument and the
//! guest issues any hostfs op whose payload exceeds a single IKC frame. We keep
//! this test minimal -- no snapshot -- and just drive the smallest sequence that
//! trips the regression.
//!
//! Reproduction sequence:
//!
//!     nanvixd.exe -ramfs nanvix_rootfs.img -- mount-multipart-test.initrd
//!     # in the guest:
//!     mount("", "/mnt", "hostfs", 0)
//!     openat(AT_FDCWD, "<long path under /mnt>", ...)   # x N
//!     write(STDOUT, "ok")
//!
//! With `hostfs_tx = None` on the host (no `-mount` arg), `standalone_io_handler`
//! answers every `SystemCallMessagePart` frame via `send_hostfs_error`. A long-path
//! op is N frames, so N error responses come back with op_ids `id`, `id|(1<<16)`,
//! `id|(2<<16)`, ... -- vfsd's pending table only matches one of them and the
//! requester stays blocked. The guest never reaches the "ok" write and `nanvixd`
//! never exits. On a fixed host this test finishes in a few seconds.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::sys::error::Error;
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
    sys_stat::file_mode::{
        S_IRUSR,
        S_IWUSR,
    },
    unistd::STDOUT_FILENO,
};
use ::syscall::unistd;

//==================================================================================================
// Constants
//==================================================================================================

/// Long path. Well over the 36-byte legacy inline-path limit, so the request is
/// fragmented by vfsd's long-message assembler into `SystemCallMessagePart` frames
/// and reassembled (or error-responded to) host-side -- exactly the code path the
/// regression rides on.
const LONG_PATH: &str =
    "/mnt/issue-2401-very-long-path-prefix/very-long-file-name-to-force-multipart.dat";

/// How many long-path ops to drive before exiting. A single op is enough to trip the
/// "surplus error response" pattern; a handful makes the regression trip reliably.
const NUM_OPS: usize = 8;

//==================================================================================================
// Main Function
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!("mount-multipart-test: starting issue #2401 regression test");

    // Phase 1: mount hostfs at /mnt. Succeeds without a host worker because vfsd's
    // mount handler is purely local (just flips an `is_enabled` flag).
    ::syscall::sys::mount::mount("", "/mnt", "hostfs", 0)?;
    ::syslog::info!("mount-multipart-test: hostfs mounted (local-only; no host worker)");

    // Phase 2: drive a small batch of long-path hostfs ops. With `hostfs_tx = None`
    // on the host, every fragment is responded to by `send_hostfs_error` -- the
    // multi-frame request meets a multi-frame error response, vfsd consumes the
    // first reply and the surplus replies trigger the regression. We do not check
    // the result of each openat: the assertion is purely that the guest keeps
    // making progress and the host eventually exits cleanly (the runner's
    // per-test timeout catches the regression's hang).
    let mode: c_int = (S_IRUSR | S_IWUSR) as c_int;
    for _ in 0..NUM_OPS {
        if let Ok(fd) =
            ::syscall::fcntl::openat(AT_FDCWD, LONG_PATH, O_CREAT | O_WRONLY | O_TRUNC, mode as u32)
        {
            let _ = ::syscall::unistd::close(fd);
        }
        if let Ok(fd) = ::syscall::fcntl::openat(AT_FDCWD, LONG_PATH, O_RDONLY, 0) {
            let _ = ::syscall::unistd::close(fd);
        }
    }
    ::syslog::info!("mount-multipart-test: long-path traffic complete");

    // Emit the magic string. The integration runner uses this to confirm the guest
    // reached the exit path; the host should then exit `nanvixd.exe` itself within a
    // few seconds. On a regressed host the guest blocks earlier in Phase 2 so the
    // magic string is never written, stdout stays empty, and the runner's per-test
    // timeout fires.
    unistd::write(STDOUT_FILENO, b"ok")?;

    Ok(())
}
