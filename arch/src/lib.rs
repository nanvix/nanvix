// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![cfg_attr(not(test), no_std)]

//==================================================================================================
// Modules
//==================================================================================================

extern crate alloc;

#[macro_use]
mod macros;

mod x86;

//==================================================================================================
// Exports
//==================================================================================================

pub use x86::*;