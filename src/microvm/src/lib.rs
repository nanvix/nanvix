// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # MicroVM
//!
//! MicroVM is a ultra-lightweight virtual machine that is designed to run the
//! [Nanvix](https://github.com/nanvix/) operating system. Currently Linux KVM is supported as
//! backend.
//!

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Macros
//==================================================================================================

/// Use this macro to add the current scope to profiling.
#[allow(unused)]
#[macro_export]
macro_rules! timer {
    ($name:expr) => {
        #[cfg(feature = "profiler")]
        let _guard = ::profiler::PROFILER.with(|p| p.borrow_mut().sync_scope($name));
    };
}

//==================================================================================================
// Modules
//==================================================================================================

pub mod args;
mod elf;
mod gateway;
mod vmm;

//==================================================================================================
// Imports
//==================================================================================================

// Must come first.
#[macro_use]
extern crate log;

#[cfg(target_os = "linux")]
extern crate kvm_bindings;
#[cfg(target_os = "linux")]
extern crate kvm_ioctls;

pub use crate::{
    gateway::Gateway,
    vmm::Vmm,
};
