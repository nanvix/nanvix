// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//! # nanvix-shim-proto
//!
//! Shimv2 protocol layer for the Nanvix containerd shim.
//!
//! This crate owns `main()`, implements the containerd shimv2 binary protocol
//! (start/delete/serve commands), creates the ttrpc server, and registers
//! both Task and Sandbox services.

//==================================================================================================
// Public Modules
//==================================================================================================

pub mod args;
pub mod executor;
pub mod sandbox_service;
pub mod task_service;

//==================================================================================================
// Private Modules
//==================================================================================================

mod sys;
