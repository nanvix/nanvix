// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(any(feature = "daemon", test))]
mod daemon;
mod message;
#[cfg(any(feature = "syscall", test))]
mod syscall;

pub mod identity;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(any(feature = "daemon", test))]
extern crate alloc;
#[cfg(all(test, not(any(feature = "daemon", feature = "syscall"))))]
extern crate log as syslog;

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of wait requests that the process manager daemon keeps blocked at the same time.
/// A blocking `waitpid()` issued while this many requests are blocked fails with `ENOBUFS`. A
/// request no longer counts once it completes, is cancelled, or its thread or process terminates.
pub const MAX_BLOCKED_WAITERS: usize = ::config::kernel::MAX_THREADS * 4;

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
    terminal_detach_ack,
    terminal_detach_request,
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
    TerminalDetachAckMessage,
    TerminalDetachMessage,
    TerminalSignalMessage,
    WaitCancelMessage,
    WaitCancelResponseMessage,
    WaitMessage,
    WaitResponseMessage,
    WaitTarget,
};

#[cfg(any(feature = "syscall", test))]
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

#[cfg(any(feature = "daemon", test))]
pub use daemon::ProcessDaemon;
