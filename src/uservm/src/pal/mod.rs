// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Platform Abstraction Layer
//!
//! This module provides a collection of platform-specific functionalities.
//!

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "windows")]
pub use self::windows::*;
