// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

/// Architecture-specific definitions.
pub mod arch;

/// m configuration constants.
pub mod config;

/// System constants.
pub mod constants;

/// Error codes.
pub use ::error;

/// Events.
pub mod event;

/// Inter process communication.
pub mod ipc;

/// Kernel calls.
#[cfg(all(target_os = "none", feature = "kcall"))]
pub mod kcall;

/// Helper macros.
pub mod macros;

/// Memory management.
pub mod mm;

/// Numbers for kernel calls.
pub mod number;

/// Process management.
pub mod pm;
