// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "none")]
mod message;
#[cfg(target_os = "none")]
mod syscall;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(target_os = "none")]
pub use self::{
    message::{
        lookup_response,
        signup_response,
        LookupMessage,
        ProcessManagementMessage,
        ProcessManagementMessageHeader,
        ShutdownMessage,
        SignupMessage,
        SignupResponseMessage,
    },
    syscall::{
        lookup,
        shutdown,
        signup,
    },
};

#[cfg(target_os = "none")]
pub use ::sys::kcall::pm::{
    capctl,
    exit,
    getegid,
    geteuid,
    getgid,
    getpid,
    gettid,
    getuid,
    terminate,
};

pub use ::sys::pm::*;
