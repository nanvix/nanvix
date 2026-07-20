// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
//! # nanvix-shim-standalone
//!
//! Standalone workload runtime for the Nanvix containerd shim.
//!
//! In standalone mode, the entire Nanvix kernel runs inside a single VM via `nanvixd.elf`.
//! The shim invokes `mkramfs.elf` to build a FAT32 filesystem image from the OCI layers,
//! then launches `nanvixd.elf` with the initrd binary and ramfs image.

pub mod mode;

#[cfg_attr(unix, path = "sys/unix/process.rs")]
#[cfg_attr(windows, path = "sys/windows/process.rs")]
mod process;

pub use mode::StandaloneRuntime;
