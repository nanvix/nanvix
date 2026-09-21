// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Backend-independent filesystem algorithms. Filesystem access, path storage, and security policy
//! belong to callers; this crate does not allocate or perform I/O.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "rustc-dep-of-std")]
#[allow(unused_extern_crates)]
extern crate compiler_builtins;
#[cfg(feature = "rustc-dep-of-std")]
#[allow(unused_extern_crates)]
extern crate core;

pub mod path;
