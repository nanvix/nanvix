// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod error;
pub mod io;
pub mod query;
pub mod socket;
mod types;

//==================================================================================================
// NetBackend
//==================================================================================================

///
/// # Description
///
/// Platform-agnostic networking backend.
///
/// This struct encapsulates all platform-specific networking logic and provides a clean Rust API
/// for socket operations. On Linux, it calls libc directly. On Windows, it uses the Winsock2 API.
///
/// `NetBackend` is designed to be shared between `linuxd` and the future `networkd` daemon without
/// code duplication.
#[derive(Clone, Copy)]
pub struct NetBackend(());

impl NetBackend {
    /// Creates a new `NetBackend` instance.
    pub fn new() -> Self {
        Self(())
    }
}

impl Default for NetBackend {
    fn default() -> Self {
        Self::new()
    }
}
