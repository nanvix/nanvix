// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Backend-independent filesystem algorithms. Filesystem access and security policy belong to
//! callers; this crate performs no I/O. Path traversal leaves path storage to callers and does not
//! allocate, whereas path normalization returns owned strings.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;
#[cfg(feature = "rustc-dep-of-std")]
#[allow(unused_extern_crates)]
extern crate compiler_builtins;
#[cfg(feature = "rustc-dep-of-std")]
#[allow(unused_extern_crates)]
extern crate core;

pub mod path;
