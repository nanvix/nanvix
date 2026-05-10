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
//! This crate defines the `ExecutionMode` trait — the primary extension point for
//! supporting different Nanvix deployment modes (standalone, distributed, etc.).

pub mod config;
pub mod execution;
pub mod registry;
pub mod state;

pub use config::NanvixRuntimeConfig;
pub use execution::ExecutionMode;
pub use registry::create_execution_mode;
pub use state::WorkloadState;
