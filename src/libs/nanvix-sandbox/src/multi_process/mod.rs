// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Multi-process sandbox implementation.
//!
//! This module provides sandboxing functionality where Linux Daemon and User VM instances
//! are spawned as separate processes. This is the default mode of operation for Nanvix Daemon.

//==================================================================================================
// Modules
//==================================================================================================

pub mod linuxd;
pub mod netns_exec;
pub mod uservm;
