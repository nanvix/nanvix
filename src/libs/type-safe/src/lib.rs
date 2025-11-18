// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Type-Safe
//!
//! This crate provides a collection of data structures that ensure type safety.
//! These structures enforce strong typing rules at compile-time and are designed
//! to avoid panics at runtime, promoting safer and more reliable code.
//!

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "std", feature(test))]
#![deny(clippy::all)]
#![deny(clippy::expect_used)]
#![deny(clippy::unwrap_used)]

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;
#[cfg(feature = "rustc-dep-of-std")]
#[allow(unused_extern_crates)]
extern crate compiler_builtins;
#[cfg(feature = "rustc-dep-of-std")]
#[allow(unused_extern_crates)]
extern crate core;

//==================================================================================================
// Modules
//==================================================================================================

mod unaligned_pointer;

mod vec_deque;

//==================================================================================================
// Exports
//==================================================================================================

pub use unaligned_pointer::*;
pub use vec_deque::*;
