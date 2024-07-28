// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
#![feature(error_in_core)] // error uses this.
#![feature(never_type)] // exit() uses this.
#![no_std]

//==================================================================================================
// Modules
//==================================================================================================

/// System configuration constants.
pub mod config;

/// System constants.
pub mod constants;

/// Error types.
pub mod error;

/// Events.
pub mod event;

/// Inter process communication.
pub mod ipc;

/// Memory management.
pub mod mm;

/// Numbers for kernel calls.
pub mod number;

/// Process management.
pub mod pm;
