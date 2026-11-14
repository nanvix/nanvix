// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
// exit() uses never_type, which Rust 1.100 stabilized, but Verus ships its own toolchain, which may
// be older and still need the gate. Allowing stable_features keeps it from failing the build once
// Verus moves to Rust 1.100 or later; drop both attributes then.
#![cfg_attr(all(feature = "kcall", verus_keep_ghost), allow(stable_features))]
#![cfg_attr(all(feature = "kcall", verus_keep_ghost), feature(never_type))]
#![cfg_attr(feature = "kcall", feature(likely_unlikely))] // Branch hints for unlikely error paths.
#![cfg_attr(feature = "kcall", feature(linkage))] // Unified per-image getpid() cache (CACHED_PID).
#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

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
