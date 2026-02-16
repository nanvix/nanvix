// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Multi-process sandbox implementation.
//!
//! This module provides sandboxing functionality where Linux Daemon and User VM instances
//! are spawned as separate processes. This is the default mode of operation for Nanvix Daemon.

//==================================================================================================
// Imports
//==================================================================================================

use crate::netns::NetnsInfo;
use ::std::{
    ffi::CString,
    io,
    os::unix::io::RawFd,
};
use ::log::error;
use ::tokio::process::Command;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Join a named network namespace by calling setns() on /var/run/netns/<name>.
///
/// This method is intended to be called inside the target process, but before calling `exec`. We
/// can achieve this behaviour using a `pre_exec` hook, as explained in detail below.
///
/// # Arguments
///
/// - `ns_name`: name of the network namespace to enter.
///
/// # Safety
///
/// This function is unsafe because it does some low-level handling of raw file descriptors. In
/// addition, it is inserted as a pre-exec hook in tokio's command, which is also an unsafe
/// operation.
///
unsafe fn setns_by_name(ns_name: &str) -> io::Result<()> {
    let ns_path: String = format!("/var/run/netns/{}", ns_name);

    // Open with O_CLOEXEC so it doesn't leak into the exec'd program.
    let c_path: CString = CString::new(ns_path.clone()).map_err(|_| {
        let reason: String = format!("invalid namespace path (path={ns_path})");
        error!("setns_by_name(): {reason}");
        io::Error::new(io::ErrorKind::InvalidInput, reason)
    })?;

    let fd: RawFd = libc::open(c_path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC);
    if fd < 0 {
        let open_err: io::Error = io::Error::last_os_error();
        error!("setns_by_name(): error opening netns file (error={open_err:?})");
        return Err(open_err);
    }

    // Join the network namespace.
    // Note: setns() returns 0 on success, -1 on error (errno set).
    let rc: libc::c_int = libc::setns(fd, libc::CLONE_NEWNET);
    let saved_err: Option<io::Error> = if rc != 0 {
        Some(io::Error::last_os_error())
    } else {
        None
    };

    // Close fd regardless.
    let close_rc: libc::c_int = libc::close(fd);
    if close_rc != 0 {
        let close_err: io::Error = io::Error::last_os_error();
        error!("setns_by_name(): error closing netns file descriptor (error={close_err:?})");
    }

    if let Some(e) = saved_err {
        error!("setns_by_name(): error entering network namespace (name={ns_name}, error={e:?})");
        return Err(e);
    }

    Ok(())
}

///
/// # Description
///
/// Spawn a program inside a network namespace.
///
/// This function spawns the provided program inside the provided network namespace without
/// requiring `sudo ip netns exec`. This function relies on executing a hook inside the new process
/// but before calling `exec`. This can be done using a `pre_exec` hook as exposed by tokio's
/// `Command` [1].
///
/// Avoiding the call to `sudo` reduces the overhead of executing a program inside a network
/// namespace, but forces the caller to have `CAP_SYS_ADMIN` + `CAP_NET_ADMIN` privileges.
///
/// [1] https://docs.rs/tokio/latest/tokio/process/struct.Command.html#method.pre_exec
///
/// # Arguments
///
/// - `info`: information on the network namespace.
/// - `program`: binary to execute inside the namespace.
/// - `args`: arguments to pass to the program.
///
/// # Returns
///
/// A Command with the right hook that can be spawned.
///
pub fn command_in_netns(info: &NetnsInfo, program: &str, args: &[String]) -> Command {
    let ns_name: String = info.ns_name().to_string();

    let mut cmd: Command = Command::new(program);
    cmd.args(args);
    // Ensure the child process is killed if the Child handle is dropped without explicit cleanup.
    // This acts as a best-effort safety net during normal unwinding and shutdown paths where drop
    // handlers run, helping to prevent orphaned processes.
    cmd.kill_on_drop(true);

    // SAFETY: inside the `pre-exec` closure we only run the logic to open the network namespace
    // file descriptor and call `setns` on it. It does not allocate any memory, and it only calls
    // async-safe functions: open, setns, and close.
    unsafe {
        cmd.pre_exec(move || {
            setns_by_name(&ns_name)?;
            Ok(())
        });
    }

    cmd
}
