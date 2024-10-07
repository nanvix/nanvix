// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Exports
//==================================================================================================

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
