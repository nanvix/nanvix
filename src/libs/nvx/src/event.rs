// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Exports
//==================================================================================================

pub use ::sys::event::*;

#[cfg(target_os = "none")]
pub use ::sys::kcall::event::{
    evctrl,
    resume,
};
