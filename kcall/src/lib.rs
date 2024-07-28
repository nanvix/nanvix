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

#[cfg(target_arch = "x86")]
#[path = "arch/x86.rs"]
mod arch;

/// Configuration constants.
pub use ::sys::config;

/// Debug facilities.
pub mod debug;

/// Event handling kernel calls.
pub mod event;

/// Inter-Process Communication (IPC) kernel calls.
pub mod ipc;

/// Memory management kernel calls.
pub mod mm;

/// Process management kernel calls.
pub mod pm;
