// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
//! # nanvix-shim-core
//!
//! Core abstractions for the Nanvix containerd shim.
//!
//! This crate defines the workload runtime interface used by the standalone container shim.

pub mod config;
pub mod runtime;
pub mod state;

pub use config::NanvixRuntimeConfig;
pub use runtime::WorkloadRuntime;
pub use state::WorkloadState;
