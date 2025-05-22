// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

/// Configuration constants.
pub mod config;

/// Error codes.
pub use ::error;

/// Events.
pub mod event;

/// Inter process communication.
pub mod ipc;

/// Kernel calls.
#[cfg(feature = "kcall")]
pub mod kcall;

/// Memory management.
pub mod mm;

/// Numbers for kernel calls.
pub mod number;

/// Process management.
pub mod pm;

/// Time management.
pub mod time;
