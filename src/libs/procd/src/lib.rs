// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(feature = "daemon")]
mod daemon;
mod message;
#[cfg(feature = "syscall")]
mod syscall;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "daemon")]
extern crate alloc;

//==================================================================================================
// Exports
//==================================================================================================

/// Process identifier of the process manager daemon.
pub const PROCD: ProcessIdentifier = ProcessIdentifier::INITD;

pub use message::{
    lookup_request,
    lookup_response,
    shutdown_request,
    signup_request,
    signup_response,
    LookupMessage,
    LookupResponseMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    ShutdownMessage,
    SignupMessage,
    SignupResponseMessage,
};

use nvx::pm::ProcessIdentifier;
#[cfg(feature = "syscall")]
pub use syscall::{
    lookup,
    signup,
};

#[cfg(feature = "daemon")]
pub use daemon::ProcessDaemon;
