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
    io,
    process::Stdio,
};
use ::tokio::process::{
    Child,
    Command,
};

//==================================================================================================
// Standalone Functions
//
// FIXME(#1171): implement these stubs with low-level libc calls + CAP_NET_ADMIN to avoid having to
// call `sudo` on the critical path, as it is known to add upwards of 10 ms of latency.
//==================================================================================================

///
/// # Description
///
/// Constructs the command arguments for executing a program within a network namespace.
///
/// # Parameters
///
/// - `info`: Network namespace information.
/// - `program`: Path to the program to execute.
/// - `args`: Arguments to pass to the program.
///
/// # Returns
///
/// A vector of strings representing the full command arguments.
///
pub fn netns_command_args(info: &NetnsInfo, program: &str, args: &[String]) -> Vec<String> {
    let mut netns_command_args: Vec<String> = vec![
        "sudo".to_string(),
        "ip".to_string(),
        "netns".to_string(),
        "exec".to_string(),
        info.ns_name().to_string(),
        program.to_string(),
    ];
    netns_command_args.extend(args.iter().cloned());
    netns_command_args
}

///
/// # Description
///
/// Creates a Tokio command configured to execute a program within a network namespace.
///
/// # Parameters
///
/// - `info`: Network namespace information.
/// - `program`: Path to the program to execute.
/// - `args`: Arguments to pass to the program.
///
/// # Returns
///
/// A configured `Command` ready to be spawned.
///
pub fn command_in_netns(info: &NetnsInfo, program: &str, args: &[String]) -> Command {
    let netns_command_args: Vec<String> = netns_command_args(info, program, args);
    let mut cmd: Command = Command::new(&netns_command_args[0]);
    cmd.args(&netns_command_args[1..]);
    cmd
}

///
/// # Description
///
/// Spawns a process within a network namespace with inherited stdout and stderr.
///
/// # Parameters
///
/// - `info`: Network namespace information.
/// - `program`: Path to the program to execute.
/// - `args`: Arguments to pass to the program.
///
/// # Returns
///
/// A `Child` process handle on success.
///
/// # Errors
///
/// Returns an I/O error if the process cannot be spawned.
///
pub async fn spawn_in_netns(
    info: &NetnsInfo,
    program: &str,
    args: &[String],
) -> io::Result<Child> {
    let mut cmd: Command = command_in_netns(info, program, args);
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    cmd.spawn()
}
