// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod elf32;

//==================================================================================================
// Feature-gated relocation support (requires goblin)
//==================================================================================================

#[cfg(feature = "relocation")]
mod relocation;

#[cfg(feature = "relocation")]
pub use relocation::*;
