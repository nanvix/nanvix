//! # nanvix-shim-core
//!
//! Core abstractions for the Nanvix containerd shim.
//!
//! This crate defines the `ExecutionMode` trait — the primary extension point for
//! supporting different Nanvix deployment modes (standalone, Hyperlight, distributed, etc.).

pub mod execution;
pub mod config;
pub mod registry;
pub mod state;

pub use execution::ExecutionMode;
pub use config::NanvixRuntimeConfig;
pub use registry::create_execution_mode;
pub use state::WorkloadState;
