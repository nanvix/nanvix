// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
#![feature(never_type)] // exit() uses this.
#![feature(likely_unlikely)] // Branch hints for unlikely error paths.
#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Modules
//==================================================================================================

// Exit status.
mod exit_status;

/// System configuration constants.
mod sys;

//==================================================================================================
// Exports
//==================================================================================================

pub use exit_status::*;
pub use sys::*;
