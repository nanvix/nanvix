// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Nanvix Crate
//!
//! The `nanvix` crate provides a unified interface for interacting with the Nanvix operating
//! system. It consolidates essential system libraries and components into a single, convenient
//! entry point for application development.
//!
//! ## Overview
//!
//! This crate serves as the primary interface for user-space applications running on Nanvix. It
//! re-exports key functionality from various system libraries, including configuration management,
//! hardware topology, HTTP communication, process sandboxing, terminal interaction, and system
//! logging.
//!
//! ## Features
//!
//! ### Default Features
//!
//! By default, the crate provides access to core system functionality:
//!
//! - **Configuration Management** (`config`) - System configuration parsing and management
//! - **Hardware Topology** (`hwloc`) - Hardware locality and topology information
//! - **HTTP Communication** (`http`) - HTTP client and server functionality
//! - **Registry** (`registry`) - Service registry for component discovery
//! - **Sandbox** (`sandbox`) - Process sandboxing and isolation
//! - **Sandbox Cache** (`sandbox_cache`) - Caching layer for sandboxed processes
//! - **Terminal** (`terminal`) - Terminal and console interaction
//! - **System Communication** (`syscomm`) - Inter-process communication primitives
//! - **System Logging** (`log`) - Structured logging facilities
//!
//! ### Optional Features
//!
//! #### `internal-api`
//!
//! Exposes low-level system APIs intended for internal use by system daemons and privileged
//! components:
//!
//! - `sys` - Low-level system interfaces
//! - `syscall` - System call wrappers
//! - `uservm` - User virtual machine management
//!
//! **Warning:** This feature provides direct access to unsafe system interfaces and should only
//! be used by trusted system components.
//!
//! #### `single-process`
//!
//! Enables single-process execution mode where all daemons run within a single process. This
//! feature is useful for development, testing, and resource-constrained environments.
//!
//! #### `hyperlight`
//!
//! Enables support for the Hyperlight virtualization backend, providing integration with
//! Hyperlight-based sandboxing and isolation mechanisms.
//!
//! ## Architecture
//!
//! The Nanvix operating system follows a microkernel architecture where core functionality is
//! distributed across specialized daemons and libraries. This crate provides the user-space
//! interface to these components, abstracting the underlying complexity and providing a clean,
//! composable API.
//!
//! ## Safety
//!
//! Most of the re-exported APIs are safe to use from user-space applications. However, when the
//! `internal-api` feature is enabled, additional unsafe interfaces become available. Applications
//! using these internal APIs must ensure they maintain memory safety and uphold system invariants.
//!
//! ## See Also
//!
//! - [`config`] - Configuration management
//! - [`hwloc`] - Hardware topology
//! - [`http`] - HTTP communication
//! - [`registry`] - Service registry
//! - [`sandbox`] - Process sandboxing
//! - [`sandbox_cache`] - Sandbox caching
//! - [`terminal`] - Terminal interaction
//! - [`syscomm`] - System communication
//! - [`log`] - System logging

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use config;
pub use hwloc;
pub use nanvix_http as http;
pub use nanvix_registry as registry;
pub use nanvix_sandbox as sandbox;
#[cfg(feature = "single-process")]
pub use nanvix_sandbox::simple_cache as sandbox_cache;
#[cfg(feature = "multi-process")]
pub use nanvix_sandbox_cache as sandbox_cache;
pub use nanvix_terminal as terminal;
pub use syscomm;
pub use syslog as log;

#[cfg(feature = "internal-api")]
pub use ::sys;
#[cfg(feature = "internal-api")]
pub use ::syscall;
#[cfg(feature = "internal-api")]
pub use uservm;
