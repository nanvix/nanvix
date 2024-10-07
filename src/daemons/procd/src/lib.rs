// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]

//==================================================================================================
// Modules
//==================================================================================================

mod message;
mod syscall;

//==================================================================================================
// Exports
//==================================================================================================

pub use message::{
    lookup_response,
    signup_response,
    LookupMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    ShutdownMessage,
    SignupMessage,
};

pub use syscall::{
    lookup,
    shutdown,
    signup,
};
