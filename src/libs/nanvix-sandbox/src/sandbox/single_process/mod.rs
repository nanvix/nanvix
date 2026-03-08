// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Single-process sandbox implementation.
//!
//! This module provides sandboxing functionality where Linux Daemon and User VM instances
//! are spawned as tasks within the same process. This mode is primarily used for testing
//! and development purposes.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(feature = "single-process")]
pub mod linuxd;
pub mod uservm;
