// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(target_os = "none")]
pub use ::sys::kcall::pm::{
    capctl,
    exit,
    getpid,
    gettid,
    gettime,
    sleep,
    terminate,
};

pub use ::sys::pm::*;
