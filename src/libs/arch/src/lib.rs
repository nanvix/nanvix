// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![allow(clippy::all)]

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use x86::*;
