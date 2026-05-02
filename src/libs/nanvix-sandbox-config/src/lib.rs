// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Sandbox configuration structures for Nanvix.
//!
//! This crate provides the configuration types used by the various sandbox deployment modes:
//! multi-process (`SandboxCacheConfig`), single-process (`SimpleSandboxCacheConfig`), and
//! standalone (`StandaloneConfig`).

//==================================================================================================
// Public Modules
//==================================================================================================

mod multi_process;
#[cfg(feature = "single-process")]
mod single_process;
mod standalone;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "single-process")]
pub use self::single_process::SimpleSandboxCacheConfig;
pub use self::{
    multi_process::SandboxCacheConfig,
    standalone::StandaloneConfig,
};
