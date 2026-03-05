// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![allow(clippy::all)]

#[cfg(target_arch = "x86")]
mod x86;
#[cfg(target_arch = "x86_64")]
mod x86_64;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(target_arch = "x86")]
pub use x86::*;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;
