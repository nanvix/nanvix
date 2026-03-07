// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Sandbox implementations.
//!
//! This module contains the process-mode-specific sandbox implementations for Linux Daemon and
//! User VM management, selected at compile time via feature flags.

//==================================================================================================
// Modules
//==================================================================================================

mod gateway_ready;

::cfg_if::cfg_if! {
    if #[cfg(feature = "single-process")] {
        pub mod single_process;
    } else {
        pub mod multi_process;
    }
}
