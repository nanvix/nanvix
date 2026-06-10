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
    fork_clone_request,
    fork_sync_ack,
    fork_sync_request,
    lookup_request,
    lookup_response,
    shutdown_request,
    signup_request,
    signup_response,
    ForkCloneMessage,
    ForkSyncMessage,
    LookupMessage,
    LookupResponseMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    ShutdownMessage,
    SignupMessage,
    SignupResponseMessage,
};

#[cfg(feature = "syscall")]
pub use syscall::{
    getegid,
    geteuid,
    getgid,
    getuid,
    lookup,
    signup,
};

#[cfg(feature = "daemon")]
pub use daemon::ProcessDaemon;
