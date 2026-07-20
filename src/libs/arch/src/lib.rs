// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![allow(clippy::all)]
// To support attributes on statements (e.g., inline `proof!{}` blocks) under Verus,
// we need `proc_macro_hygiene`.
#![cfg_attr(verus_keep_ghost, feature(proc_macro_hygiene))]

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86;

#[cfg(target_arch = "x86_64")]
mod x86_64;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use x86::*;
