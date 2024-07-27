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

/// Debug facilities.
mod debug;

/// Kernel error types.
mod error;

/// Event handling kernel calls.
mod event;

/// Inter-Process Communication (IPC) kernel calls.
mod ipc;

/// Memory management kernel calls.
mod mm;

/// Numbers for kernel calls.
mod number;

/// Process management kernel calls.
mod pm;

//==================================================================================================
// Exports
//==================================================================================================

pub use arch::*;
pub use debug::*;
pub use error::*;
pub use event::*;
pub use ipc::*;
pub use mm::*;
pub use number::*;
pub use pm::*;
