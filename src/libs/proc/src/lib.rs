// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(feature = "daemon")]
mod daemon;
mod message;
#[cfg(feature = "syscall")]
mod syscall;

pub mod identity;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "daemon")]
extern crate alloc;

//==================================================================================================
// Exports
//==================================================================================================

pub use message::{
    exec_ack,
    exec_request,
    fork_clone_ack,
    fork_clone_request,
    fork_sync_ack,
    fork_sync_request,
    job_control_request,
    job_control_response,
    kill_request,
    kill_response,
    lookup_request,
    lookup_response,
    process_exit_request,
    shutdown_request,
    signup_request,
    signup_response,
    terminal_access_request,
    terminal_signal_request,
    wait_cancel_request,
    wait_cancel_response,
    wait_request,
    wait_response,
    ExecAckMessage,
    ExecMessage,
    ForkCloneAckMessage,
    ForkCloneMessage,
    ForkSyncAckMessage,
    ForkSyncMessage,
    JobControlOp,
    JobControlRequest,
    JobControlResponse,
    KillMessage,
    KillResponseMessage,
    LookupMessage,
    LookupResponseMessage,
    ProcessExitMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    ShutdownMessage,
    SignupMessage,
    SignupResponseMessage,
    TerminalAccessMessage,
    TerminalSignalMessage,
    WaitCancelMessage,
    WaitCancelResponseMessage,
    WaitMessage,
    WaitResponseMessage,
    WaitTarget,
};

#[cfg(feature = "syscall")]
pub use syscall::{
    getegid,
    geteuid,
    getgid,
    getpgid,
    getpgrp,
    getsid,
    getuid,
    kill,
    lookup,
    setpgid,
    setsid,
    signup,
    tcgetpgrp,
    tcsetpgrp,
    wait,
    WaitOutcome,
};

#[cfg(feature = "daemon")]
pub use daemon::ProcessDaemon;
