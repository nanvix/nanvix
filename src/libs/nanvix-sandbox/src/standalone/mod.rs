// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Standalone sandbox implementation.
//!
//! This module provides sandboxing functionality where the User VM runs without connecting to
//! a system VM, control-plane, or gateway. The VM's stdout messages are drained and discarded;
//! the VM's stderr is captured as usual. This mode is useful for debugging and local testing.

//==================================================================================================
// Modules
//==================================================================================================

pub mod uservm;
