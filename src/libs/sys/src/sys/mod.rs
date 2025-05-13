// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

/// Architecture-specific definitions.
pub mod arch;

/// Configuration constants.
pub mod config;

/// Error codes.
pub use ::error;

/// Events.
pub mod event;

/// Inter process communication.
pub mod ipc;

/// Kernel calls.
#[cfg(all(target_os = "none", feature = "kcall"))]
pub mod kcall;

/// Memory management.
pub mod mm;

/// Numbers for kernel calls.
pub mod number;

/// Process management.
pub mod pm;
